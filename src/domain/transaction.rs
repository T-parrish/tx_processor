use super::{errors::TransactionError, Account, TryUpdate};
use rust_decimal::Decimal;

#[derive(Debug, serde::Deserialize, Default, PartialEq)]
pub struct Transaction {
    #[serde(rename="type")]
    pub op: Operation,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug, serde::Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Operation {
    #[default]
    Deposit,
    Withdrawal,
    Resolve,
    Chargeback,
    Dispute,
}

impl TryUpdate<&mut Account> for &Transaction {
    type Output = ();
    type Error = TransactionError;

    fn try_update(self, rhs: &mut Account) -> Result<Self::Output, Self::Error> {
        if rhs.locked {
            return Err(TransactionError::LockedAccount)
        }

        match self.op {
            Operation::Deposit => rhs.deposit(self.amount),
            Operation::Withdrawal => rhs.withdraw(self.amount),
            // need to retrieve the disputed transaction to properly
            // update held / available balances, assuming that this transaction
            // is populated with the appropriate amount
            Operation::Resolve => rhs.resolve(self.amount),
            Operation::Chargeback => rhs.chargeback(self.amount),
            Operation::Dispute => rhs.dispute(self.amount),
        }
    }
}

#[cfg(test)]
pub mod test {
    use rust_decimal_macros::dec;

    use crate::domain::errors::TransactionError;

    use super::*;

    #[test]
    fn successful_deposit() {
        let tx: Transaction = Transaction {
            op: Operation::Deposit,
            client: 1,
            tx: 1,
            amount: Some(dec!(42)),
        };

        let mut act = Account {
            client: 1,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        };

        let out = Account {
            client: 1,
            available: dec!(42),
            held: dec!(0.0),
            total: dec!(42),
            locked: false,
        };

        tx.try_update(&mut act).expect("Failed to update Account");

        assert_eq!(act, out);
    }

    #[test]
    fn insufficient_funds_for_withdrawal() {
        let tx: Transaction = Transaction {
            op: Operation::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(dec!(42)),
        };

        let mut act = Account {
            client: 1,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        };

        let res = tx.try_update(&mut act);

        match res {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, TransactionError::InsufficientFunds)
        }
    }
    
    #[test]
    fn successful_withdrawal() {
        let tx: Transaction = Transaction {
            op: Operation::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(dec!(42)),
        };

        let mut act = Account {
            client: 1,
            available: dec!(42),
            held: dec!(0.0),
            total: dec!(42),
            locked: false,
        };

        let out = Account {
            client: 1,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        };

        let res = tx.try_update(&mut act);

        match res {
            Ok(_) => {
                assert!(true);
                assert_eq!(act, out)
            },
            Err(_) => assert!(false)
        }
    }
}
