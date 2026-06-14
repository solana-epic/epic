use anchor_lang::prelude::*;
#[account]
pub struct SimpleAccount {
    pub data: u64,
}
