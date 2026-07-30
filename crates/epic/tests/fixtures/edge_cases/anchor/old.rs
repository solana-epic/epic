use anchor_lang::prelude::*;

#[account]
pub struct OrderedStruct {
    pub value1: u64,
    pub value2: u64,
}
