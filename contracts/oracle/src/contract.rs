use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::PoolResponse;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{query_pair, query_pool};
use crate::state::{PAIR_INFO, POOL_INFO};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdatePair {
            token,
            denom1,
            denom2,
        } => update_pair(deps, env, token, denom1, denom2),
        ExecuteMsg::UpdatePool {
            total_share,
            amount1,
            amount2,
        } => update_pool(deps, total_share, amount1, amount2),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    env: Env,
    msg: astroport::pair::QueryMsg,
) -> Result<Binary, ContractError> {
    match msg {
        astroport::pair::QueryMsg::Pair {} => Ok(to_json_binary(&query_pair(deps)?)?),
        astroport::pair::QueryMsg::Pool {} => Ok(to_json_binary(&query_pool(deps)?)?),
        _ => unimplemented!(),
    }
}

pub fn update_pair(
    deps: DepsMut,
    env: Env,
    token: String,
    denom1: String,
    denom2: String,
) -> Result<Response, ContractError> {
    let new_pair_info = PairInfo {
        asset_infos: vec![
            AssetInfo::NativeToken { denom: denom1 },
            AssetInfo::NativeToken { denom: denom2 },
        ],
        contract_addr: env.contract.address,
        liquidity_token: deps.api.addr_validate(&token)?,
        pair_type: astroport::factory::PairType::Xyk {},
    };

    PAIR_INFO.save(deps.storage, &new_pair_info)?;

    Ok(Response::new())
}

pub fn update_pool(
    deps: DepsMut,
    total_share: Uint128,
    amount1: Uint128,
    amount2: Uint128,
) -> Result<Response, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let new_pool_info = PoolResponse {
        assets: vec![
            Asset {
                info: pair_info.asset_infos[0].clone(),
                amount: amount1,
            },
            Asset {
                info: pair_info.asset_infos[1].clone(),
                amount: amount2,
            },
        ],
        total_share,
    };

    POOL_INFO.save(deps.storage, &new_pool_info)?;

    Ok(Response::new())
}

#[cfg(test)]
pub mod test {}
