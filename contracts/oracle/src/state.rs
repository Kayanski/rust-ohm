// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cw_storage_plus::Item;

pub const PAIR_INFO: Item<astroport::asset::PairInfo> = Item::new("pair");
pub const POOL_INFO: Item<astroport::pair::PoolResponse> = Item::new("pool");
