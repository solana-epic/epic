use anchor_lang::prelude::*;

#[account]
pub struct Multisig {
    pub create_key: Pubkey,
    pub config_authority: Pubkey,
    pub threshold: u16,
}
