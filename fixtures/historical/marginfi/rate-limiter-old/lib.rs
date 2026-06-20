use anchor_lang::prelude::*;

#[account]
pub struct Bank {
    pub mint: Pubkey,
    pub mint_decimals: u8,
    pub integration_acc_1: Pubkey,
    pub integration_acc_2: Pubkey,
    pub integration_acc_3: Pubkey,
    pub padding_1: [u64; 26],
}

#[account]
pub struct MarginfiGroup {
    pub admin: Pubkey,
    pub padding: [u8; 8],
    pub padding_0: [u64; 22],
    pub padding_1: [u64; 64],
}
