pub mod account;
pub mod transaction;
pub mod errors;
pub mod tx_history;

pub use account::Account;
pub use transaction::Transaction;
pub use tx_history::History;

pub trait TryUpdate<Rhs> {
    type Output;
    type Error;
    // Required method
    fn try_update(self, rhs: Rhs) -> Result<(), Self::Error> ;
}