// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

pub const STAKING_TOKEN_DENOM: &str = "staking_token";

#[cw_serde]
pub struct Config {
    pub ohm: String,  // Native token denom
    pub sohm: String, // Staking denom created by contract
    pub admin: Addr,
}
