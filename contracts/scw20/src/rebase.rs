use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};

use crate::{query::circulating_supply, state::REBASE_CONFIG, ContractError};

pub const INITIAL_FRAGMENTS_SUPPLY: u128 = 5_000_000;
pub const TOTAL_GONS: u128 = u128::MAX - (u128::MAX % INITIAL_FRAGMENTS_SUPPLY);
pub const MAX_SUPPLY: u128 = u64::MAX as u128;

pub fn execute_rebase(
    deps: DepsMut,
    info: MessageInfo,
    profit: Uint128,
) -> Result<Response, ContractError> {
    if profit.is_zero() {
        return Ok(Response::new());
    }

    // Auth
    let mut rebase_config = REBASE_CONFIG.load(deps.storage)?;
    if info.sender != rebase_config.staking_contract {
        return Err(ContractError::UnauthorizedWithAddress {
            address: rebase_config.staking_contract,
        });
    }
    // End - Auth

    // Computations
    let circulating_supply = circulating_supply(deps.as_ref())?;
    let mut token_info = cw20_base::state::TOKEN_INFO.load(deps.storage)?;

    let rebase_amount = if !circulating_supply.is_zero() {
        profit * token_info.total_supply / circulating_supply
    } else {
        profit
    };

    token_info.total_supply += rebase_amount;

    if token_info.total_supply > Uint128::from(MAX_SUPPLY) {
        token_info.total_supply = Uint128::from(MAX_SUPPLY);
    }

    rebase_config.gons_per_fragment = Uint128::from(TOTAL_GONS) / token_info.total_supply;
    // End Computations

    //State modifications
    REBASE_CONFIG.save(deps.storage, &rebase_config)?;
    cw20_base::state::TOKEN_INFO.save(deps.storage, &token_info)?;
    //End modifications

    Ok(Response::new())
}
