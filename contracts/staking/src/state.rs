// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Timestamp};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");
pub const EPOCH_STATE: Item<EpochState> = Item::new("epoch_state");

pub const STAKING_TOKEN_DENOM: &str = "staking_token";
pub const BASE_TOKEN_DENOM: &str = "base_token";

#[cw_serde]
pub struct Config {
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub admin: Addr,
}

#[cw_serde]
pub struct EpochState {
    pub epoch_end: Timestamp,
    pub epoch_number: u64,
}
