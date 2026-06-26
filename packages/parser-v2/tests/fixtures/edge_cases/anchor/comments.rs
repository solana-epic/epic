use anchor_lang::prelude::*;

const S: &str = "http://example.com";
#[account]
pub struct Real {
    pub value: u64,
}
