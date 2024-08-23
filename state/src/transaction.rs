use std::collections::HashSet;
use crate::state_record::StateRecord;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use solana_sdk::hash::Hash;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::{Message, MessageHeader};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::{SanitizedTransaction, Transaction};
use std::io::{Error, ErrorKind};

impl StateRecord for TrollupTransaction {
    fn get_key(&self) -> [u8; 32] {
        let hash: [u8; 32] = Sha256::digest(&self.signatures[0]).into();
        hash
    }
}

// Wrapper structures for Borsh serialization
#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct TrollupTransaction {
    pub optimistic: bool,
    pub signatures: Vec<[u8; 64]>,
    pub message: TrollupMessage,
}

pub fn message_header_to_bytes(message_header: &MessageHeader) -> [u8; 3] {
    [
        message_header.num_required_signatures,
        message_header.num_readonly_signed_accounts,
        message_header.num_readonly_unsigned_accounts,
    ]
}

// Create MessageHeader from [u8; 3]
pub fn message_header_from_bytes(bytes: [u8; 3]) -> MessageHeader {
    MessageHeader {
        num_required_signatures: bytes[0],
        num_readonly_signed_accounts: bytes[1],
        num_readonly_unsigned_accounts: bytes[2],
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct TrollupMessage {
    pub header: [u8; 3],
    pub account_keys: Vec<[u8; 32]>,
    pub recent_blockhash: [u8; 32],
    pub instructions: Vec<TrollupCompileInstruction>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct TrollupCompileInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

// Conversion functions
impl From<&Transaction> for TrollupTransaction {
    fn from(tx: &Transaction) -> Self {
        let mut sigs: Vec<[u8; 64]> = Vec::with_capacity(tx.signatures.len());
        for sig in &tx.signatures {
            let sig_bytes: [u8; 64] = sig.clone().into();
            sigs.push(sig_bytes);
        }
        TrollupTransaction {
            optimistic: false,
            signatures: sigs,
            message: (&tx.message).into(),
        }
    }
}

impl From<&Message> for TrollupMessage {
    fn from(msg: &Message) -> Self {
        TrollupMessage {
            header: message_header_to_bytes(&msg.header.clone()),
            account_keys: msg.account_keys.iter().map(|key| key.to_bytes()).collect(),
            recent_blockhash: msg.recent_blockhash.to_bytes(),
            instructions: msg.instructions.iter().map(|ix| ix.into()).collect(),
        }
    }
}

impl From<&CompiledInstruction> for TrollupCompileInstruction {
    fn from(ix: &CompiledInstruction) -> Self {
        TrollupCompileInstruction {
            program_id_index: ix.program_id_index,
            accounts: ix.accounts.clone(),
            data: ix.data.clone(),
        }
    }
}

// Serialization function
pub fn serialize_transaction(transaction: &Transaction) -> Result<Vec<u8>, Error> {
    let borsh_tx: TrollupTransaction = transaction.into();
    to_vec(&borsh_tx).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
}

// Deserialization function
pub fn deserialize_transaction(data: &[u8]) -> Result<Transaction, Error> {
    let borsh_tx: TrollupTransaction = BorshDeserialize::deserialize(&mut &data[..])
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let mut sigs: Vec<Signature> = Vec::with_capacity(borsh_tx.signatures.len());
    for sig in borsh_tx.signatures {
        let signature: Signature = sig.into();
        sigs.push(signature);
    }
    let signatures = sigs;

    let message = Message {
        header: message_header_from_bytes(borsh_tx.message.header),
        account_keys: borsh_tx.message.account_keys.into_iter()
            .map(Pubkey::new_from_array)
            .collect(),
        recent_blockhash: Hash::new(&borsh_tx.message.recent_blockhash),
        instructions: borsh_tx.message.instructions.into_iter()
            .map(|ix| CompiledInstruction {
                program_id_index: ix.program_id_index,
                accounts: ix.accounts,
                data: ix.data,
            })
            .collect(),
    };

    Ok(Transaction {
        signatures,
        message,
    })
}

pub fn convert_to_solana_transaction(tx: TrollupTransaction) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Convert signatures
    let signatures: Vec<Signature> = tx.signatures
        .into_iter()
        .map(Signature::from)
        .collect();

    // Convert message
    let message = Message {
        header: message_header_from_bytes(tx.message.header),
        account_keys: tx.message.account_keys
            .into_iter()
            .map(Pubkey::from)
            .collect(),
        recent_blockhash: Hash::new(&tx.message.recent_blockhash),
        instructions: tx.message.instructions
            .into_iter()
            .map(|ix| CompiledInstruction {
                program_id_index: ix.program_id_index,
                accounts: ix.accounts,
                data: ix.data,
            })
            .collect(),
    };

    // Create and return the Solana Transaction
    Ok(Transaction {
        signatures,
        message,
    })
}

pub fn convert_to_trollup_transaction(tx: Transaction) -> Result<TrollupTransaction, Box<dyn std::error::Error>> {
    // Convert signatures
    // let sig_bytes: [u8; 64] = sig.clone().into();

    let signatures: Vec<[u8; 64]> = tx.signatures
        .into_iter()
        .map(|sig| sig.into())
        .collect();

    // Convert message
    let message = TrollupMessage {
        header: [tx.message.header.num_required_signatures, tx.message.header.num_readonly_signed_accounts, tx.message.header.num_readonly_unsigned_accounts],
        account_keys: tx.message.account_keys
            .into_iter()
            .map(|account_key| account_key.to_bytes())
            .collect(),
        recent_blockhash: tx.message.recent_blockhash.to_bytes(),
        instructions: tx.message.instructions
            .into_iter()
            .map(|ix| TrollupCompileInstruction {
                program_id_index: ix.program_id_index,
                accounts: ix.accounts,
                data: ix.data,
            })
            .collect(),
    };

    // Create and return the Solana Transaction
    Ok(TrollupTransaction {
        optimistic: false,
        signatures,
        message,
    })
}

pub fn convert_to_sanitized_transaction(tx: &TrollupTransaction) -> solana_sdk::transaction::Result<SanitizedTransaction> {
    let transaction = convert_to_solana_transaction(tx.clone()).expect("Error converting TrollupTransaction to Solana Transaction");
    SanitizedTransaction::try_from_legacy_transaction(transaction, &HashSet::new())
}
