use anchor_lang::prelude::*;

#[account]
pub struct Collection {
    pub name: String,
    pub items: Vec<Pubkey>,
}
