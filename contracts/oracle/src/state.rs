use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::PricesResponseElem;
use cosmwasm_std::{Addr, Decimal256, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub base_asset: String,
}
pub const CONFIG: Item<Config> = Item::new("config");

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceInfo {
    pub price: Decimal256,
    pub last_updated_time: u64,
}

pub const PRICES: Map<&str, PriceInfo> = Map::new("price_info");
pub const FEEDER: Map<&str, Addr> = Map::new("feeder");
pub fn store_price(storage: &mut dyn Storage, asset: &str, price: &PriceInfo) -> StdResult<()> {
    PRICES.save(storage, asset, price)
}

pub fn read_price(storage: &dyn Storage, asset: &str) -> StdResult<PriceInfo> {
    let res = PRICES.load(storage, asset);
    match res {
        Ok(data) => Ok(data),
        Err(_err) => Err(StdError::generic_err(
            "No price data for the specified asset exist",
        )),
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_prices(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<PricesResponseElem>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_ref().map(|s| Bound::exclusive(s.as_str()));

    PRICES
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (asset, v) = item?;

            Ok(PricesResponseElem {
                asset,
                price: v.price,
                last_updated_time: v.last_updated_time,
            })
        })
        .collect()
}

pub fn store_feeder(storage: &mut dyn Storage, asset: &str, feeder: &Addr) -> StdResult<()> {
    FEEDER.save(storage, asset, feeder)
}

pub fn read_feeder(storage: &dyn Storage, asset: &str) -> StdResult<Addr> {
    let res = FEEDER.load(storage, asset);
    match res {
        Ok(data) => Ok(data),
        Err(_err) => Err(StdError::generic_err(
            "No feeder data for the specified asset exist",
        )),
    }
}
