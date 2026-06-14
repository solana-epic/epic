use anchor_lang::prelude::*;
pub struct NestedStruct {
    pub val: u128,
}
#[account]
pub struct ParentAccount {
    pub nested: NestedStruct,
}
