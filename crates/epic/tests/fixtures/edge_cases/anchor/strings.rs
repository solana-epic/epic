use anchor_lang::prelude::*;

const FAKE: &str = r#"
#[account]
pub struct FakeAccount {
    pub value: u64,
}
"#;

#[account]
pub struct RealAccount {
    pub value: u64,
}
