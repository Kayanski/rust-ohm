// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKETS: Item<Vec<Market>> = Item::new("markets");
pub const TERMS: Item<Term> = Item::new("terms");
pub const STAKING_TOKEN_DENOM: &str = "staking_token";
pub const CURRENT_DEBT: Item<u128> = Item::new("current_debt");
pub const LAST_DECAY: Item<u128> = Item::new("last_decay");

#[cw_serde]
pub struct Config {
    pub ohm: String, // Native token denom
    pub principle: Addr,
    pub treasury: Addr,
    pub dao: Addr,
    pub feed: Addr,
    pub admin: Addr,
}

#[cw_serde]
pub struct Market {
    pub quote_token: Addr,
    pub capacity_in_quote: bool,
    pub capacity: u128,
    pub total_debt: u128,
    pub max_payout: u128,
    pub sold: u128,
    pub purchased: u128,
}

#[cw_serde]
pub struct Term {
    pub control_variable: u128,
    pub minimum_price: Decimal256,
    pub max_payout: u128,
    pub max_debt: u128,
    pub vesting_term: u128,
}

#[cw_serde]
pub struct Bond {
    pub payout: u128,
    pub price_paid: Decimal256,
    pub vesting_time_left: u128,
    pub last_time: u128,
}
