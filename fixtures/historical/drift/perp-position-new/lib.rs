use anchor_lang::prelude::*;

#[account]
pub struct PerpPosition {
    pub base_asset_amount: i64,
    pub quote_asset_amount: i64,
    pub lp_shares: u64,
    pub isolated_position_scaled_balance: u64,
    pub last_quote_asset_amount_per_lp: i64,
    pub market_index: u16,
    pub open_orders: u8,
    pub position_flag: u8,
}
