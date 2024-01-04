use crate::execute::{lock, unlock};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;
use crate::msg::{AcceptedTokenUnchecked, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{lock_for_token, locks_for_address, query_accepted_tokens, query_config};
use crate::state::{Config, ACCEPTED_TOKENS, CONFIG};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        admin: msg
            .admin
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .unwrap_or(info.sender),
    };

    msg.accepted_tokens
        .into_iter()
        .try_for_each(|asset_unchecked| {
            let asset = asset_unchecked.check(deps.api)?;
            ACCEPTED_TOKENS.save(deps.storage, &asset.asset, &asset.deposit_fee)?;

            Ok::<_, ContractError>(())
        })?;

    CONFIG.save(deps.storage, &config)?;

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
        ExecuteMsg::UpdateConfig { admin } => update_config(deps, info, admin),
        ExecuteMsg::UpdateAcceptedToken { to_add, to_remove } => {
            update_accepted_token(deps, info, to_add, to_remove)
        }
        ExecuteMsg::Lock { to, asset } => lock(deps, env, info, to, asset),
        ExecuteMsg::Unlock { to, asset } => unlock(deps, env, info, to, asset),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::AcceptedTokens { start, limit } => Ok(to_json_binary(&query_accepted_tokens(
            deps, env, start, limit,
        )?)?),
        QueryMsg::LockForToken { address, token } => {
            Ok(to_json_binary(&lock_for_token(deps, env, address, token)?)?)
        }
        QueryMsg::LocksForAddress {
            address,
            start,
            limit,
        } => Ok(to_json_binary(&locks_for_address(
            deps, env, address, start, limit,
        )?)?),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}
pub fn update_accepted_token(
    deps: DepsMut,
    info: MessageInfo,
    to_add: Vec<AcceptedTokenUnchecked>,
    to_remove: Vec<cw_asset::AssetInfoBase<String>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    to_add.iter().try_for_each(|a| {
        let checked_token = a.check(deps.api)?;
        ACCEPTED_TOKENS.save(
            deps.storage,
            &checked_token.asset,
            &checked_token.deposit_fee,
        )?;
        Ok::<_, ContractError>(())
    })?;

    to_remove.iter().try_for_each(|a| {
        let checked_token = a.check(deps.api, None)?;
        ACCEPTED_TOKENS.remove(deps.storage, &checked_token);
        Ok::<_, ContractError>(())
    })?;

    Ok(Response::new())
}
