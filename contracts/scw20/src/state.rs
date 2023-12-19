// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Addr};
use cw_storage_plus::Item;

pub const REBASE_CONFIG: Item<RebaseConfig> = Item::new("rebase_config");

#[cw_serde]
pub struct RebaseConfig {
    pub gons_per_fragment: Uint128,
    pub staking_contract: Addr,
}
