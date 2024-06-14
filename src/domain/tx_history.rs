use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{transaction::Operation, Transaction};

#[derive(Debug, Default)]
pub struct History {
    // K = tuple of client, tx mapped to Node
    history: HashMap<(u16, u32), Node>,
}

impl History {
    pub fn new() -> Self {
        Self {
            history: HashMap::<(u16, u32), Node>::new(),
        }
    }
    pub fn insert(&mut self, tx: &Transaction) -> Option<Node> {
        let node = Node::from(tx);
        self.history.insert((tx.client, tx.tx), node)
    }
    pub fn get(&self, key: &(u16, u32)) -> Option<&Node> {
        self.history.get(key)
    }
}

#[derive(Debug)]
pub struct Node {
    pub op: Operation,
    pub amount: Option<Decimal>,
}

impl From<&Transaction> for Node {
    fn from(value: &Transaction) -> Self {
        Self {
            op: value.op.clone(),
            amount: value.amount,
        }
    }
}
