use anchor_lang::prelude::*;

#[account]
pub struct OrderedStruct {
    pub value2: u64,
    pub value1: u64,
}
