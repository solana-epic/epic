use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct NestedData {
    pub value1: u64,
    pub value2: u64,
}

#[account]
pub struct MyAccount {
    pub authority: Pubkey,
    pub data: NestedData,
}
