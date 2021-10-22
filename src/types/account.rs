use crate::types::Balance;
use ed25519_dalek::{PublicKey};

#[derive(Debug, Clone)]
pub enum AccountType {
    User,
    Contract,
}

#[derive(Debug, Clone)]
pub struct Account {
    account_type: AccountType,
    pub(crate) balance: Balance,
    pub(crate) public_key: PublicKey,
}

impl Account {
    pub fn new(account_type: AccountType, public_key: PublicKey) -> Self {
        Self {
            account_type,
            balance: 0,
            public_key,
        }
    }
}
