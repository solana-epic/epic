use anchor_lang::prelude::*;
pub type CustomId = Pubkey;
#[account]
pub struct AliasAccount {
    pub owner: CustomId,
}
