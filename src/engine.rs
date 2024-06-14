use std::collections::HashMap;

use rust_decimal_macros::dec;

use crate::domain::{
    errors::TransactionError, transaction::Operation, tx_history::History, Account, Transaction,
    TryUpdate,
};

#[derive(Debug)]
pub enum State {
    Idle,
    Fetching,
    Updating,
    Logging,
    Done,
}

pub struct Task<'a> {
    history: &'a mut History,
    accounts: &'a mut HashMap<u16, Account>,
    transaction: Transaction,
    state: State,
}

impl<'a> Task<'a> {
    pub fn new(
        history: &'a mut History,
        accounts: &'a mut HashMap<u16, Account>,
        transaction: Transaction,
    ) -> Self {
        Self {
            history,
            accounts,
            transaction,
            state: State::Idle,
        }
    }
}

impl<'a> Machine for Task<'a> {
    fn run(&mut self) -> Result<(), TransactionError> {
        loop {
            match self.state {
                State::Idle => match self.transaction.op {
                    // if the transaction is a deposit or a withdrawal, attempt to apply transaction to account
                    Operation::Deposit | Operation::Withdrawal => {
                        self.state = State::Updating;
                        self.next_state()?;
                    }
                    // if the transaction is from the family of dispute operations, fetch the associated
                    // transaction from the transaction history.
                    Operation::Resolve | Operation::Chargeback | Operation::Dispute => {
                        self.state = State::Fetching;
                        self.next_state()?;
                    }
                },
                State::Fetching | State::Updating | State::Logging => {
                    self.next_state()?;
                }
                State::Done => return Ok(()),
            }
        }
    }

    fn next_state(&mut self) -> Result<&mut Self, TransactionError> {
        match self.state {
            State::Idle => Ok(self),
            State::Fetching => {
                // For disputes, fetch the disputed transaction from the history
                let maybe_node = self
                    .history
                    .get(&(self.transaction.client, self.transaction.tx));
                if let Some(node) = maybe_node {
                    // set the disputed amount on the dispute transaction, reversing deposits should be
                    // negative and reversing withdrawals should be positive.
                    match node.op {
                        Operation::Deposit => {
                            self.transaction.amount = node.amount.map(|el| dec!(-1) * el)
                        }
                        Operation::Withdrawal => self.transaction.amount = node.amount,
                        _ => self.transaction.amount = node.amount,
                    };
                    self.state = State::Updating;
                    Ok(self)
                } else {
                    Err(TransactionError::TransactionNotFound)
                }
            }
            State::Updating => {
                let maybe_account = self.accounts.get_mut(&self.transaction.client);
                if let Some(act) = maybe_account {
                    self.transaction.try_update(act)?;
                } else {
                    let mut new_act = Account::new(self.transaction.client);
                    self.transaction.try_update(&mut new_act)?;
                    self.accounts.insert(self.transaction.client, new_act);
                }
                self.state = State::Logging;
                Ok(self)
            }
            State::Logging => {
                // Mutates tx node to reflect most recent op (ie Deposit, Dispute, Chargeback...)
                // or inserts a new history Node
                self.history.insert(&self.transaction);
                self.state = State::Done;
                Ok(self)
            }
            State::Done => Ok(self),
        }
    }
}

pub trait Machine {
    fn run(&mut self) -> Result<(), TransactionError>;
    fn next_state(&mut self) -> Result<&mut Self, TransactionError>;
}

#[cfg(test)]
pub mod test {
    use crate::domain::tx_history::History;
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn handles_deposit() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let transaction = Transaction {
            op: Operation::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(10)),
        };
        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(10),
            held: dec!(0.0),
            total: dec!(10),
            locked: false,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), expected);
    }

    #[test]
    fn handles_successful_withdrawal() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(40),
            held: dec!(0.0),
            total: dec!(40),
            locked: false,
        };
        accounts.insert(1, start);

        let transaction = Transaction {
            op: Operation::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(dec!(20)),
        };
        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(20),
            held: dec!(0.0),
            total: dec!(20),
            locked: false,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), expected);
    }

    #[test]
    fn handles_failed_withdrawal() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(40),
            held: dec!(0.0),
            total: dec!(40),
            locked: false,
        };
        accounts.insert(1, start);

        let transaction = Transaction {
            op: Operation::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(dec!(50)),
        };
        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_err());
        match result {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, TransactionError::InsufficientFunds),
        }
    }

    #[test]
    fn handles_dispute() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);
        let tx0 = Transaction {
            client: 1,
            tx: 1,
            op: Operation::Withdrawal,
            amount: Some(dec!(50)),
        };

        let mut task0 = Task::new(&mut history, &mut accounts, tx0);
        task0.run().expect("Failed task");

        let tx1 = Transaction {
            op: Operation::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(100),
            held: dec!(50),
            total: dec!(150),
            locked: false,
        };
        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), expected);
    }

    #[test]
    fn handles_dispute_and_chargeback() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);
        let tx0 = Transaction {
            client: 1,
            tx: 1,
            op: Operation::Withdrawal,
            amount: Some(dec!(50)),
        };

        let mut task0 = Task::new(&mut history, &mut accounts, tx0);
        task0.run().expect("Failed task");

        let tx1 = Transaction {
            op: Operation::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(100),
            held: dec!(50),
            total: dec!(150),
            locked: false,
        };
        {
            let output = accounts.get(&1);
            assert!(output.is_some());
            assert_eq!(*output.unwrap(), expected);
        }

        let tx2 = Transaction {
            op: Operation::Chargeback,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task2 = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx2,
            state: State::Idle,
        };

        let res2 = task2.run();
        assert!(res2.is_ok());

        let final_expected = Account {
            client: 1,
            available: dec!(100),
            held: dec!(0),
            total: dec!(100),
            locked: true,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), final_expected);
    }

    #[test]
    fn handles_dispute_and_resolve() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);
        let tx0 = Transaction {
            client: 1,
            tx: 1,
            op: Operation::Withdrawal,
            amount: Some(dec!(50)),
        };

        let mut task0 = Task::new(&mut history, &mut accounts, tx0);
        task0.run().expect("Failed task");

        task0.run().expect("Failed initial withdrawal");

        let tx1 = Transaction {
            op: Operation::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(100),
            held: dec!(50),
            total: dec!(150),
            locked: false,
        };
        {
            let output = accounts.get(&1);
            assert!(output.is_some());
            assert_eq!(*output.unwrap(), expected);
        }

        let tx2 = Transaction {
            op: Operation::Resolve,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task2 = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx2,
            state: State::Idle,
        };

        let res2 = task2.run();
        assert!(res2.is_ok());

        let final_expected = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), final_expected);
    }

    #[test]
    fn handles_dispute_and_resolve_deposit() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);
        let tx0 = Transaction {
            client: 1,
            tx: 1,
            op: Operation::Deposit,
            amount: Some(dec!(50)),
        };

        let mut task0 = Task::new(&mut history, &mut accounts, tx0);
        task0.run().expect("Failed initial deposit");

        let tx1 = Transaction {
            op: Operation::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(150),
            held: dec!(50),
            total: dec!(200),
            locked: false,
        };
        {
            let output = accounts.get(&1);
            assert!(output.is_some());
            assert_eq!(*output.unwrap(), expected);
        }

        let tx2 = Transaction {
            op: Operation::Resolve,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task2 = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx2,
            state: State::Idle,
        };

        let res2 = task2.run();
        assert!(res2.is_ok());

        let final_expected = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), final_expected);
    }

    #[test]
    fn handles_dispute_and_chargeback_deposit() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);
        let tx0 = Transaction {
            client: 1,
            tx: 1,
            op: Operation::Deposit,
            amount: Some(dec!(50)),
        };

        let mut task0 = Task::new(&mut history, &mut accounts, tx0);
        task0.run().expect("Failed initial deposit");

        let tx1 = Transaction {
            op: Operation::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let result = task.run();
        assert!(result.is_ok());

        let expected = Account {
            client: 1,
            available: dec!(150),
            held: dec!(50),
            total: dec!(200),
            locked: false,
        };
        {
            let output = accounts.get(&1);
            assert!(output.is_some());
            assert_eq!(*output.unwrap(), expected);
        }

        let tx2 = Transaction {
            op: Operation::Chargeback,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task2 = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx2,
            state: State::Idle,
        };

        let res2 = task2.run();
        assert!(res2.is_ok());

        let final_expected = Account {
            client: 1,
            available: dec!(200),
            held: dec!(0),
            total: dec!(200),
            locked: true,
        };

        let output = accounts.get(&1);
        assert!(output.is_some());
        assert_eq!(*output.unwrap(), final_expected);
    }

    #[test]
    fn no_active_dispute() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: false,
        };
        accounts.insert(1, start);

        let tx1 = Transaction {
            op: Operation::Chargeback,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let res = task.run();

        assert!(res.is_err());
        match res {
            Ok(_) => assert!(false),
            Err(ref e) => assert_eq!(*e, TransactionError::TransactionNotFound),
        };

        let tx2 = Transaction {
            op: Operation::Resolve,
            client: 1,
            tx: 1,
            amount: None,
        };

        let mut task2 = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx2,
            state: State::Idle,
        };

        let res2 = task2.run();

        assert!(res2.is_err());
        match res {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, TransactionError::TransactionNotFound),
        }
    }

    #[test]
    fn locked_account() {
        let mut history = History::new();
        let mut accounts = HashMap::<u16, Account>::new();
        let start = Account {
            client: 1,
            available: dec!(150),
            held: dec!(0),
            total: dec!(150),
            locked: true,
        };
        accounts.insert(1, start);

        let tx1 = Transaction {
            op: Operation::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(100)),
        };

        let mut task = Task {
            history: &mut history,
            accounts: &mut accounts,
            transaction: tx1,
            state: State::Idle,
        };

        let res = task.run();

        assert!(res.is_err());
        match res {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, TransactionError::LockedAccount),
        }
    }
}
