use super::errors::TransactionError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serializer;

#[derive(Debug, serde::Deserialize, serde::Serialize, Default, PartialEq)]
pub struct Account {
    pub client: u16,
    // Total - held
    #[serde(serialize_with="four_decimal_precision")]
    pub available: Decimal,
    // total - available
    #[serde(serialize_with="four_decimal_precision")]
    pub held: Decimal,
    // available + held
    #[serde(serialize_with="four_decimal_precision")]
    pub total: Decimal,
    pub locked: bool,
}

pub fn four_decimal_precision<S>(decimal: &Decimal, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = decimal.round_dp(4).to_string();
    s.serialize_str(&rounded)
}



impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        }
    }

    pub fn withdraw(&mut self, amt: Option<Decimal>) -> Result<(), TransactionError> {
        match amt {
            Some(val) if val > self.available => Err(TransactionError::InsufficientFunds),
            Some(val) if val <= self.available => {
                self.total -= val;
                self.available = self.total - self.held;
                self.held = self.total - self.available;
                Ok(())
            }
            None => Ok(()),
            _ => Err(TransactionError::UnspecifiedBehavior),
        }
    }

    pub fn deposit(&mut self, amt: Option<Decimal>) -> Result<(), TransactionError> {
        // Deposits should always have an amount, if missing default to 0.0
        self.total += amt.unwrap_or_default();
        self.available = self.total - self.held;
        self.held = self.total - self.available;
        Ok(())
    }

    pub fn resolve(&mut self, amt: Option<Decimal>) -> Result<(), TransactionError> {
        let val = amt.unwrap_or_default();
        // if resolving deposit dispute
        if val < dec!(0) {
            self.held += val;
            self.total += val;
        // if resolving withdrawal dispute
        } else {
            self.held -= val;
            self.available += val;
        }
        Ok(())
    }

    pub fn chargeback(&mut self, amt: Option<Decimal>) -> Result<(), TransactionError> {
        let val = amt.unwrap_or_default();
        // if charging back deposit dispute
        if val < dec!(0) {
            self.held += val;
            self.available -= val;
        // if charging back withdrawal dispute
        } else {
            self.held -= val;
            self.total -= val;
        }
        self.locked = true;
        Ok(())
    }

    pub fn dispute(&mut self, amt: Option<Decimal>) -> Result<(), TransactionError> {
        let val = amt.unwrap_or_default();
        // if disputing deposit
        if val < dec!(0) {
            self.held -= val;
            self.available += val;
        } else {
            // if disputing withdrawal
            self.held += val;
            self.total += val;
        }
        Ok(())
    }
}
