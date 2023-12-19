// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Timestamp, Uint128};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");
pub const EPOCH_STATE: Item<EpochState> = Item::new("epoch_state");

pub const STAKING_TOKEN_DENOM: &str = "staking_token";

#[cw_serde]
pub struct Config {
    pub ohm: String, // Native token denom
    pub epoch_length: u64,
    pub admin: Addr,
}

#[cw_serde]
pub struct EpochState {
    pub number: u64,
    pub end: Timestamp,
    pub distribute: Uint128,
    pub current_exchange_rate: Decimal256,
}
