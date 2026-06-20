use anchor_lang::prelude::*;

#[account]
pub struct Multisig {
    pub create_key: Pubkey,
    pub config_authority: Pubkey,
    pub threshold: u16,
}

#[account]
pub struct TransactionBuffer {
    pub multisig: Pubkey,
    pub creator: Pubkey,
    pub vault_index: u8,
    pub transaction_index: u64,
    pub final_buffer_hash: [u8; 32],
    pub final_buffer_size: u16,
    pub buffer: Vec<u8>,
}
