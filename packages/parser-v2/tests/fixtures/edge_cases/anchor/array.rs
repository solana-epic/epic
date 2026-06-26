use anchor_lang::prelude::*;

#[account]
pub struct ArrayAccount {
    pub data1: [u8; 32],
    pub data2: [u8; 32 ],
}
