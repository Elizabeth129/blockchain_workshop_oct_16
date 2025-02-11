use crate::traits::{Hashable, WorldState};
use crate::types::{Account, AccountId, AccountType, Block, Chain, Error, Hash, Transaction};
use ed25519_dalek::{Keypair, Signature, Signer, PublicKey};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Blockchain {
    target: u128,
    blocks: Chain<Block>,
    accounts: HashMap<AccountId, Account>,
    transaction_pool: Vec<Transaction>,
}

impl WorldState for Blockchain {
    fn create_account(
        &mut self,
        account_id: AccountId,
        account_type: AccountType,
        public_key: PublicKey,
    ) -> Result<(), Error> {
        match self.accounts.entry(account_id.clone()) {
            Entry::Occupied(_) => Err(format!("AccountId already exist: {}", account_id)),
            Entry::Vacant(v) => {
                v.insert(Account::new(account_type, public_key));
                Ok(())
            }
        }
    }

    fn get_account_by_id(&self, account_id: AccountId) -> Option<&Account> {
        self.accounts.get(&account_id)
    }

    fn get_account_by_id_mut(&mut self, account_id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(&account_id)
    }
}

impl Blockchain {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn append_block(&mut self, block: Block) -> Result<(), Error> {
        //TODO Task 3: Implement mining

        if !block.verify() {
            return Err("Block has invalid hash".to_string());
        }
        let is_genesis = self.blocks.len() == 0;

        if block.transactions.len() == 0 {
            return Err("Block has 0 transactions.".to_string());
        }

        let account_backup = self.accounts.clone();
        for tx in &block.transactions {
            let res = tx.execute(self, is_genesis);
            if let Err(error) = res {
                self.accounts = account_backup;
                return Err(format!("Error during tx execution: {}", error));
            }
        }

        // TODO Task 3: Append block only if block.hash < target
        // Adjust difficulty of target each block generation (epoch)
        if is_genesis
        {
            self.target = 0x00000000ffff0000000000000000000000000000;
        }
        else if block.hash().parse::<u128>().unwrap() >= self.target
        {
            return Err("The hash of block more than target.".to_string());
        }

        let first_transaction = block.transactions[0].timestamp;
        let last_transaction = block.transactions[block.transactions.len() - 1].timestamp;
        let actual = last_transaction - first_transaction;
        let expected = block.transactions.len() * 10 * 60;
        let mut ratio = ((actual as f64)/(expected as f64)) as u128;
        if ratio > 4 
        {
            ratio = 4;
        }
        let new_target = self.target*ratio;
        if new_target > 0x00000000ffff0000000000000000000000000000
        {
            self.target = 0x00000000ffff0000000000000000000000000000;
        }
        else
        {
            self.target = new_target;
        }

        self.blocks.append(block);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), Error> {
        let mut block_num = self.blocks.len();
        let mut prev_block_hash: Option<Hash> = None;

        for block in self.blocks.iter() {
            let is_genesis = block_num == 1;

            if !block.verify() {
                return Err(format!("Block {} has invalid hash", block_num));
            }

            if !is_genesis && block.prev_hash.is_none() {
                return Err(format!("Block {} doesn't have prev_hash", block_num));
            }

            if is_genesis && block.prev_hash.is_some() {
                return Err("Genesis block shouldn't have prev_hash".to_string());
            }

            if block_num != self.blocks.len() {
                if let Some(prev_block_hash) = &prev_block_hash {
                    if prev_block_hash != &block.hash.clone().unwrap() {
                        return Err(format!(
                            "Block {} prev_hash doesn't match Block {} hash",
                            block_num + 1,
                            block_num
                        ));
                    }
                }
            }

            prev_block_hash = block.prev_hash.clone();
            block_num -= 1;
        }

        Ok(())
    }

    pub fn get_last_block_hash(&self) -> Option<Hash> {
        self.blocks.head().map(|block| block.hash())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TransactionData;
    use crate::utils::{append_block, append_block_with_tx};

    #[test]
    fn test_new() {
        let bc = Blockchain::new();
        assert_eq!(bc.get_last_block_hash(), None);
    }

    #[test]
    fn test_append() {
        let bc = &mut Blockchain::new();

        append_block(bc, 1);
        let block = append_block(bc, 2);

        assert_eq!(bc.get_last_block_hash(), block.hash);
    }

    #[test]
    fn test_create_genesis_block() {
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let bc = &mut Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );

        let mut block = Block::new(None);
        block.add_transaction(tx_create_account);
        block.add_transaction(tx_mint_initial_supply);

        let mut i = 1;
        while i < 10000
        {
            block.set_nonce(i);
            if u128::from_str_radix(&block.hash(), 16).ok().unwrap() < bc.target
            {
                break;
            }
            i += 1;
        }
        /*
        if block.hash() < bc.target
        {
            assert!(
                bc.append_block(block).is_ok()
            );

            let satoshi = bc.get_account_by_id("satoshi".to_string());

            assert!(satoshi.is_some());
            assert_eq!(satoshi.unwrap().balance, 100_000_000);
        } else
        {
            */
            assert_eq!(
                bc.append_block(block).err().unwrap(),
                "Error during tx execution: Invalid account.".to_string()
            );
        //}

        
    }

    #[test]
    fn test_create_genesis_block_fails() {
        let mut bc = Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );
        let mut block = Block::new(None);
        block.add_transaction(tx_mint_initial_supply);
        block.add_transaction(tx_create_account);

        let mut i = 1;
        while i < 10000
        {
            block.set_nonce(i);
            if block.hash().parse::<u128>().unwrap() < bc.target
            {
                break;
            }
            i += 1;
        }

        assert_eq!(
            bc.append_block(block).err().unwrap(),
            "Error during tx execution: Invalid account.".to_string()
        );        
    }

    #[test]
    fn test_state_rollback_works() {
        let mut bc = Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );
        let mut block = Block::new(None);
        block.set_nonce(1);
        block.add_transaction(tx_create_account);
        block.add_transaction(tx_mint_initial_supply);

        assert!(bc.append_block(block).is_ok());

        let mut block = Block::new(bc.get_last_block_hash());
        let keypair_alice = Keypair::generate(&mut rand::rngs::OsRng {});
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_alice =
            Transaction::new(TransactionData::CreateAccount("alice".to_string(), keypair_alice.public), None, time);
        let keypair_bob = Keypair::generate(&mut rand::rngs::OsRng {});
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_bob =
            Transaction::new(TransactionData::CreateAccount("bob".to_string(), keypair_bob.public), None, time);
        block.set_nonce(2);
        block.add_transaction(tx_create_alice);
        block.add_transaction(tx_create_bob.clone());
        block.add_transaction(tx_create_bob);

        assert!(bc.append_block(block).is_err());

        assert!(bc.get_account_by_id("satoshi".to_string()).is_some());
        assert!(bc.get_account_by_id("alice".to_string()).is_none());
        assert!(bc.get_account_by_id("bob".to_string()).is_none());
    }

    #[test]
    fn test_validate() {
        let bc = &mut Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );
        assert!(
            append_block_with_tx(bc, 1, vec![tx_create_account, tx_mint_initial_supply]).is_ok()
        );

        append_block(bc, 2);
        append_block(bc, 3);

        assert!(bc.validate().is_ok());

        let mut iter = bc.blocks.iter_mut();
        iter.next();
        iter.next();
        let block = iter.next().unwrap();
        block.transactions[1].data = TransactionData::MintInitialSupply {
            to: "satoshi".to_string(),
            amount: 100,
        };

        assert!(bc.validate().is_err());
    }

    #[test]
    fn test_transfer_transaction() {
        let mut bc = Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );

        let keypair_alice = Keypair::generate(&mut rand::rngs::OsRng {});
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_alice =
            Transaction::new(TransactionData::CreateAccount("alice".to_string(), keypair_alice.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply_alice = Transaction::new(
                TransactionData::MintInitialSupply {
                    to: "alice".to_string(),
                    amount: 100_000,
                },
                None,
                time,
        );

        let mut block = Block::new(None);
        block.set_nonce(1);
        block.add_transaction(tx_create_account);
        block.add_transaction(tx_mint_initial_supply);
        block.add_transaction(tx_create_alice);
        block.add_transaction(tx_mint_initial_supply_alice);

        assert!(bc.append_block(block).is_ok());

        let mut block = Block::new(bc.get_last_block_hash());
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let mut tx_transfer_satoshi_to_alice = Transaction::new(
                TransactionData::Transfer{
                    to: "alice".to_string(),
                    amount: 1000,
                },
                Some("satoshi".to_string()),
                time,
        );
        tx_transfer_satoshi_to_alice.sign(Some(keypair.sign(tx_transfer_satoshi_to_alice.hash().as_bytes())));

        block.set_nonce(2);
        block.add_transaction(tx_transfer_satoshi_to_alice);

        assert!(bc.append_block(block).is_ok());

        let satoshi = bc.get_account_by_id("satoshi".to_string());

        assert!(satoshi.is_some());
        assert_eq!(satoshi.unwrap().balance, 99_999_000);

        let alice = bc.get_account_by_id("alice".to_string());

        assert!(alice.is_some());
        assert_eq!(alice.unwrap().balance, 101_000);
    }

    #[test]
    fn test_transfer_transaction_fails() {
        let mut bc = Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );

        let keypair_alice = Keypair::generate(&mut rand::rngs::OsRng {});
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_alice =
            Transaction::new(TransactionData::CreateAccount("alice".to_string(), keypair_alice.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply_alice = Transaction::new(
                TransactionData::MintInitialSupply {
                    to: "alice".to_string(),
                    amount: 100_000,
                },
                None,
                time,
        );

        let mut block = Block::new(None);
        block.set_nonce(1);
        block.add_transaction(tx_create_account);
        block.add_transaction(tx_mint_initial_supply);
        block.add_transaction(tx_create_alice);
        block.add_transaction(tx_mint_initial_supply_alice);

        assert!(bc.append_block(block).is_ok());

        let mut block = Block::new(bc.get_last_block_hash());
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let mut tx_transfer_satoshi_to_alice = Transaction::new(
                TransactionData::Transfer{
                    to: "satoshi".to_string(),
                    amount: 102_000,
                },
                Some("alice".to_string()),
                time,
        );
        tx_transfer_satoshi_to_alice.sign(Some(keypair_alice.sign(tx_transfer_satoshi_to_alice.hash().as_bytes())));
        block.set_nonce(2);
        block.add_transaction(tx_transfer_satoshi_to_alice);

        assert!(bc.append_block(block).is_err());

        let satoshi = bc.get_account_by_id("satoshi".to_string());

        assert!(satoshi.is_some());
        assert_eq!(satoshi.unwrap().balance, 100_000_000);

        let alice = bc.get_account_by_id("alice".to_string());

        assert!(alice.is_some());
        assert_eq!(alice.unwrap().balance, 100_000);
    }

    #[test]
    fn test_transfer_transaction_sign_fails() {
        let mut bc = Blockchain::new();

        let keypair = Keypair::generate(&mut rand::rngs::OsRng {});
        let mut time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_account =
            Transaction::new(TransactionData::CreateAccount("satoshi".to_string(), keypair.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply = Transaction::new(
            TransactionData::MintInitialSupply {
                to: "satoshi".to_string(),
                amount: 100_000_000,
            },
            None,
            time,
        );

        let keypair_alice = Keypair::generate(&mut rand::rngs::OsRng {});
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_create_alice =
            Transaction::new(TransactionData::CreateAccount("alice".to_string(), keypair_alice.public), None, time);
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let tx_mint_initial_supply_alice = Transaction::new(
                TransactionData::MintInitialSupply {
                    to: "alice".to_string(),
                    amount: 100_000,
                },
                None,
                time,
        );

        let mut block = Block::new(None);
        block.set_nonce(1);
        block.add_transaction(tx_create_account);
        block.add_transaction(tx_mint_initial_supply);
        block.add_transaction(tx_create_alice);
        block.add_transaction(tx_mint_initial_supply_alice);

        assert!(bc.append_block(block).is_ok());

        let mut block = Block::new(bc.get_last_block_hash());
        time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs() as u128;
        let mut tx_transfer_satoshi_to_alice = Transaction::new(
                TransactionData::Transfer{
                    to: "satoshi".to_string(),
                    amount: 2_000,
                },
                Some("alice".to_string()),
                time,
        );
        tx_transfer_satoshi_to_alice.sign(Some(keypair.sign(tx_transfer_satoshi_to_alice.hash().as_bytes())));
        block.set_nonce(2);
        block.add_transaction(tx_transfer_satoshi_to_alice);

        assert!(bc.append_block(block).is_err());

        let satoshi = bc.get_account_by_id("satoshi".to_string());

        assert!(satoshi.is_some());
        assert_eq!(satoshi.unwrap().balance, 100_000_000);

        let alice = bc.get_account_by_id("alice".to_string());

        assert!(alice.is_some());
        assert_eq!(alice.unwrap().balance, 100_000);
    }
}
