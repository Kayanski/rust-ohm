// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Deps, DepsMut, Env, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use crate::{
    query::{asset_price, debt_ratio},
    ContractError,
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const TERMS: Item<Terms> = Item::new("terms");

pub const TOTAL_DEBT: Item<Uint128> = Item::new("total_debt");
pub const LAST_DECAY: Item<Timestamp> = Item::new("last_decay");
pub const ADJUSTMENT: Item<Adjustment> = Item::new("adjustment");

pub const BOND_INFO: Map<&Addr, Bond> = Map::new("bond_info");

#[cw_serde]
pub struct Config {
    pub usd: String,
    pub principle: String,
    pub admin: Addr,
    pub staking: Addr,
    pub oracle: Addr,
    pub oracle_trust_period: u64, // in Seconds
    pub treasury: Addr,
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
pub struct Terms {
    pub control_variable: Decimal256,
    pub minimum_price: Decimal256,
    pub max_payout: Decimal256,
    pub max_debt: Uint128,
    pub vesting_term: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct Bond {
    pub payout: Uint128,
    pub price_paid: Decimal256,
    pub vesting_time_left: u64,
    pub last_time: Timestamp,
}

#[cw_serde]
pub struct Adjustment {
    pub add: bool,
    pub rate: Decimal256,
    pub target: Decimal256,
    pub buffer: u64,
    pub last_time: Timestamp,
}

pub fn bond_price(deps: DepsMut, env: Env) -> Result<Decimal256, ContractError> {
    let mut terms = TERMS.load(deps.storage)?;

    let mut price = terms.control_variable * debt_ratio(deps.as_ref(), env)?;
    if price < terms.minimum_price {
        price = terms.minimum_price;
    } else if !terms.minimum_price.is_zero() {
        terms.minimum_price = Decimal256::zero();
    };

    TERMS.save(deps.storage, &terms)?;

    Ok(price)
}
pub fn query_bond_price(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    let terms = TERMS.load(deps.storage)?;

    let mut price = terms.control_variable * debt_ratio(deps, env)?;
    if price < terms.minimum_price {
        price = terms.minimum_price;
    }

    Ok(price)
}

pub fn bond_price_in_usd(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    let price = query_bond_price(deps, env.clone())? * asset_price(deps, env)?;

    Ok(price)
}
