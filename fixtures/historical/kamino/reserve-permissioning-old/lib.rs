use anchor_lang::prelude::*;

#[account]
pub struct Reserve {
    pub version: u64,
    pub lending_market: Pubkey,
    pub config_padding: [u64; 113],
    pub borrowed_amount_outside_elevation_group: u64,
}

#[account]
pub struct ReserveConfig {
    pub debt_term_seconds: u64,
    pub early_repay_remaining_interest_pct: u64,
    pub rewards_amount_per_slot: u64,
}
