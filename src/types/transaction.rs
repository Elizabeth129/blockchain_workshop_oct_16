use crate::traits::{Hashable, WorldState};
use crate::types::{AccountId, AccountType, Balance, Error, Hash, Timestamp};
use blake2::digest::FixedOutput;
use blake2::{Blake2s, Digest};

#[derive(Debug, Clone)]
pub struct Transaction {
    nonce: u128,
    timestamp: Timestamp,
    from: Option<AccountId>,
    pub(crate) data: TransactionData,
    signature: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TransactionData {
    CreateAccount(AccountId),
    MintInitialSupply { to: AccountId, amount: Balance },
    Transfer { to: AccountId, amount: Balance },
}

impl Transaction {
    pub fn new(data: TransactionData, from: Option<AccountId>) -> Self {
        Self {
            nonce: 0,
            timestamp: 0,
            from,
            data,
            signature: None,
        }
    }

    pub fn execute<T: WorldState>(&self, state: &mut T, is_genesis: bool) -> Result<(), Error> {
        //TODO Task 2: Implement signature
        match &self.data {
            TransactionData::CreateAccount(account_id) => {
                state.create_account(account_id.clone(), AccountType::User)
            }
            TransactionData::MintInitialSupply { to, amount } => {
                if !is_genesis {
                    return Err("Initial supply can be minted only in genesis block.".to_string());
                }
                match state.get_account_by_id_mut(to.clone()) {
                    Some(account) => {
                        account.balance += amount;
                        Ok(())
                    }
                    None => Err("Invalid account.".to_string()),
                }
            }
            // TODO Task 1: Implement transfer transition function
            // 1. Check that receiver and sender accounts exist
            // 2. Check sender balance
            // 3. Change sender/receiver balances and save to state
            // 4. Test
            TransactionData::Transfer { to, amount } => {
                let sender;
                let receiver;
                if matches!(state.get_account_by_id_mut(self.from.as_ref().unwrap().clone()), Some(account))
                {
                    sender = state.get_account_by_id_mut(self.from.as_ref().unwrap().clone()).unwrap().clone();
                }
                else
                {
                    return Err("Invalid sender account.".to_string());
                }
                if matches!(state.get_account_by_id_mut(to.clone()), Some(account))
                {
                    receiver = state.get_account_by_id_mut(to.clone()).unwrap().clone();
                }
                else
                {
                    return Err("Invalid receiver account.".to_string());
                }

                if sender.balance < *amount
                {
                    return Err("Insufficient balance".to_string());
                }
                if u128::MAX - *amount < receiver.balance
                {
                    return Err("Type overflow".to_string());
                }

                match state.get_account_by_id_mut(self.from.as_ref().unwrap().clone()) {
                    Some(account) => {
                        account.balance -= amount;
                    }
                    None => return Err("Invalid sender account.".to_string()),
                }

                match state.get_account_by_id_mut(to.clone()) {
                    Some(account) => {
                        account.balance += amount;
                    }
                    None => return Err("Invalid receiver account.".to_string()),
                }

                return Ok(());
            },
        }
    }
}

impl Hashable for Transaction {
    fn hash(&self) -> Hash {
        let mut hasher = Blake2s::new();

        hasher.update(format!(
            "{:?}",
            (
                self.nonce,
                self.timestamp,
                self.from.clone(),
                self.data.clone()
            )
        ));

        hex::encode(hasher.finalize_fixed())
    }
}
