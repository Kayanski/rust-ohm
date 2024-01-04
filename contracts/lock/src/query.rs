use cosmwasm_std::{Deps, Env};
use cw_asset::{AssetBase, AssetInfoBase};
use cw_storage_plus::Bound;

use crate::{
    msg::{AcceptedTokenUnchecked, ConfigResponse},
    state::{ACCEPTED_TOKENS, CONFIG, DEPOSITED_TOKENS},
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

pub fn lock_for_token(
    deps: Deps,
    _env: Env,
    address: String,
    token: AssetInfoBase<String>,
) -> Result<cw_asset::AssetBase<String>, ContractError> {
    let addr = deps.api.addr_validate(&address)?;
    let token_checked = token.check(deps.api, None)?;
    let lock_info = DEPOSITED_TOKENS
        .load(deps.storage, (&addr, &token_checked))
        .or(Err(ContractError::NoDepositInfo {
            address: addr,
            token: token_checked,
        }))?;

    Ok(cw_asset::AssetBase {
        info: token,
        amount: lock_info,
    })
}

pub fn locks_for_address(
    deps: Deps,
    _env: Env,
    address: String,
    start: Option<AssetInfoBase<String>>,
    limit: Option<u32>,
) -> Result<Vec<AssetBase<String>>, ContractError> {
    let addr = deps.api.addr_validate(&address)?;

    let start_checked = start.map(|s| s.check(deps.api, None)).transpose()?;

    cw_paginate::paginate_map_prefix(
        &DEPOSITED_TOKENS,
        deps.storage,
        &addr,
        start_checked.as_ref().map(Bound::exclusive),
        limit,
        |k, v| {
            Ok::<_, ContractError>(AssetBase {
                info: k.into(),
                amount: v,
            })
        },
    )
}
