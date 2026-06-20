use anchor_lang::prelude::*;

#[account]
pub struct Bank {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub force_withdraw: u8,
    pub tier: [u8; 4],
    pub collected_fees_native: u64,
}
