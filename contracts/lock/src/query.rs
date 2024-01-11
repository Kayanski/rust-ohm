use cosmwasm_std::{Deps, Env, Order, StdError, Uint128, Uint256};
use cw_asset::{Asset, AssetBase, AssetInfoBase};
use cw_paginate::{DEFAULT_LIMIT, MAX_LIMIT};
use cw_storage_plus::Bound;

use crate::{
    msg::{AcceptedTokenUnchecked, ConfigResponse, DepositInfoResponse},
    state::{
        deposit::{deposits, DepositInfo},
        ACCEPTED_TOKENS, CONFIG,
    },
    ContractError,
};

pub fn query_config(deps: Deps, _env: Env) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        admin: config.admin.to_string(),
    })
}

pub fn query_accepted_tokens(
    deps: Deps,
    _env: Env,
    start: Option<AssetInfoBase<String>>,
    limit: Option<u32>,
) -> Result<Vec<AcceptedTokenUnchecked>, ContractError> {
    let start_checked = start.map(|s| s.check(deps.api, None)).transpose()?;

    cw_paginate::paginate_map(
        &ACCEPTED_TOKENS,
        deps.storage,
        start_checked.as_ref().map(Bound::exclusive),
        limit,
        |k, v| {
            Ok::<_, ContractError>(AcceptedTokenUnchecked {
                asset: k.into(),
                deposit_fee: v,
            })
        },
    )
}

pub fn query_lock(deps: Deps, _env: Env, id: u64) -> Result<DepositInfoResponse, ContractError> {
    let stored_info = deposits().load(deps.storage, id)?;

    Ok(stored_info.into())
}

pub fn locks_for_address(
    deps: Deps,
    _env: Env,
    address: String,
    start: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<DepositInfoResponse>, ContractError> {
    let addr = deps.api.addr_validate(&address)?;

    let start = start.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(deposits()
        .idx
        .recipient
        .prefix(addr)
        .range(deps.storage, start, None, Order::Descending)
        .map(|r| match r {
            Err(e) => Err(e),
            Ok((_k, v)) => Ok(v.into()),
        })
        .take(limit)
        .collect::<Result<_, StdError>>()?)
}

pub fn query_available_unlock(
    deps: Deps,
    env: Env,
    id: u64,
) -> Result<AssetBase<String>, ContractError> {
    let deposit_info = deposits().load(deps.storage, id)?;

    let available_unlock = _available_unlock(deps, &env, &deposit_info)?.unwrap_or(AssetBase {
        info: deposit_info.asset.info,
        amount: Uint128::zero(),
    });

    Ok(available_unlock.into())
}

pub fn _available_unlock(
    _deps: Deps,
    env: &Env,
    info: &DepositInfo,
) -> Result<Option<Asset>, ContractError> {
    Ok(match &info.lock {
        crate::state::deposit::DepositLock::Linear(lock) => {
            if env.block.time <= lock.start {
                None
            } else {
                let available_amount =
                    Uint256::from(env.block.time.seconds() - lock.start.seconds())
                        * lock.per_second_vesting;
                let withdraw_amount = info.asset.amount.min(available_amount.try_into()?);

                Some(Asset {
                    info: info.asset.info.clone(),
                    amount: withdraw_amount,
                })
            }
        }
        crate::state::deposit::DepositLock::LinearProportion(_) => todo!(),
        crate::state::deposit::DepositLock::TimeUnlock(unlock) => {
            if unlock < &env.block.time.seconds() {
                Some(info.asset.clone())
            } else {
                None
            }
        }
    })
}
