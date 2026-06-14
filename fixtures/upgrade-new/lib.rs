use anchor_lang::prelude::*;

#[account]
pub struct Position {
    pub owner: Pubkey,
    pub score: u64,
}
