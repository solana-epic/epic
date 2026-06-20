use anchor_lang::prelude::*;

#[account]
pub struct Position {
    pub amount: u64,
    pub owner: Pubkey,
}
