use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum TransactionError {
    #[error("Insufficient funds in account")]
    InsufficientFunds,
    #[error("Cannot find transaction")]
    TransactionNotFound,
    #[error("Unexpected behavior")]
    UnspecifiedBehavior,
    #[error("Account Frozen")]
    LockedAccount
}