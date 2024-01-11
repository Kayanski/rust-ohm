use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Deps, DepsMut, Env, Order, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use crate::{
    msg::{BondContractsElem, BondContractsResponse},
    query::staking_token_balance,
    ContractError,
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const EPOCH_STATE: Item<EpochState> = Item::new("epoch_state");
pub const BOND_CONTRACT_INFO: Map<&Addr, BondContractInfo> = Map::new("minter_info");
pub const WARMUP: Map<&Addr, Warmup> = Map::new("warmup_info");
pub const STAKING_POINTS: Map<&Addr, StakingPoints> = Map::new("staking_points");

pub const BASE_TOKEN_DENOM: &str = "base_token";

#[cw_serde]
pub struct Config {
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub next_epoch_apr: Option<Decimal256>,
    pub admin: Addr,
    pub staking_denom_address: Option<Addr>,
    pub warmup_address: Option<Addr>,
    pub warmup_length: u64,
}
#[cw_serde]
pub struct EpochState {
    pub epoch_start: Timestamp,
    pub epoch_end: Timestamp,
    pub epoch_number: u64,
}

#[cw_serde]
pub struct BondContractInfo {
    pub bond_token: String,
    pub can_mint: bool,
}

#[cw_serde]
pub struct StakingPoints {
    pub total_points: Uint128,
    pub last_points_updated: Timestamp,
}

#[cw_serde]
pub struct Warmup {
    pub amount: Uint128,
    pub end: Timestamp,
    pub mint_amount: Uint128,
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

pub fn update_staking_points(
    mut deps: DepsMut,
    env: Env,
    address: &Addr,
    stake_amount: Uint128,
) -> Result<(), ContractError> {
    let current_stake = staking_token_balance(deps.as_ref(), address)?;

    STAKING_POINTS.update(deps.branch().storage, address, |points| {
        staking_points_update_closure(&env, current_stake, points)
    })?;

    Ok(())
}

pub fn staking_points_update_closure(
    env: &Env,
    current_stake: Uint128,
    staking_points: Option<StakingPoints>,
) -> Result<StakingPoints, ContractError> {
    match staking_points {
        None => Ok(StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: env.block.time,
        }),
        Some(mut staking_points) => {
            let time_delta =
                env.block.time.seconds() - staking_points.last_points_updated.seconds();
            let new_points = current_stake * Uint128::from(time_delta);

            staking_points.total_points += new_points;
            staking_points.last_points_updated = env.block.time;
            Ok(staking_points)
        }
    }
}
