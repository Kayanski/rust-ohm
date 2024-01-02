// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Deps, Order, Timestamp};
use cw_storage_plus::{Item, Map};

use crate::{
    msg::{BondContractsElem, BondContractsResponse},
    ContractError,
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const EPOCH_STATE: Item<EpochState> = Item::new("epoch_state");
pub const BOND_CONTRACT_INFO: Map<&Addr, BondContractInfo> = Map::new("minter_info");

pub const BASE_TOKEN_DENOM: &str = "base_token";

#[cw_serde]
pub struct Config {
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub admin: Addr,
    pub staking_denom_address: Option<Addr>,
}

#[cw_serde]
pub struct EpochState {
    pub epoch_end: Timestamp,
    pub epoch_number: u64,
}

#[cw_serde]
pub struct BondContractInfo {
    pub bond_token: String,
    pub can_mint: bool,
}

pub fn bond_contracts(deps: Deps) -> Result<BondContractsResponse, ContractError> {
    let active_bonds: Result<Vec<_>, _> = BOND_CONTRACT_INFO
        .range(deps.storage, None, None, Order::Descending)
        .filter(|i| match i {
            Err(_) => false,
            Ok((_, info)) => info.can_mint,
        })
        .collect();

    Ok(BondContractsResponse {
        bonds: active_bonds?
            .into_iter()
            .map(|(addr, info)| BondContractsElem {
                bond_token: info.bond_token,
                bond_address: addr.to_string(),
            })
            .collect(),
    })
}
