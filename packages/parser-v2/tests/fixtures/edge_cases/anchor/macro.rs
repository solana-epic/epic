use anchor_lang::prelude::*;

#[account]
const SOME_VAR: u8 = 5;

pub struct NotAnAccount {
    pub value: u64,
}
