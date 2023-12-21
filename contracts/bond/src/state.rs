// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Timestamp, Uint128};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKETS: Item<Vec<Market>> = Item::new("markets");
pub const TERMS: Item<Term> = Item::new("terms");

pub const CURRENT_DEBT: Item<Uint128> = Item::new("current_debt");
pub const LAST_DECAY: Item<Timestamp> = Item::new("last_decay");

#[cw_serde]
pub struct Config {
    pub principle: String,
    pub pair: Addr,
    pub admin: Addr,
    pub staking: Addr,
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
    pub control_variable: Uint128,
    pub minimum_price: Decimal256,
    pub max_payout: Uint128,
    pub max_debt: Uint128,
    pub vesting_term: u64,
}

#[cw_serde]
pub struct Bond {
    pub payout: Uint128,
    pub price_paid: Decimal256,
    pub vesting_time_left: Uint128,
    pub last_time: Uint128,
}

pub fn bondPrice(deps: Deps) -> Result<Uint128, ContractError> {
    let terms = TERMS.load(deps.storage)?;

    let price = terms.control_variable * debt_ratio();
    if price < terms.minimum_price {
        price = terms.minimum_price;
    } else if !terms.minimum_price.is_zero() {
        terms.minimum_price = Decimal256::zero();
    };

    Ok(price)
}
