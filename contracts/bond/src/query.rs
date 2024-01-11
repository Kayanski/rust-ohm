use cosmwasm_std::{to_json_binary, Addr, Decimal256, Deps, Env, SupplyResponse, Uint128, Uint256};

use crate::{
    execute::current_debt,
    state::{query_bond_price, Adjustment, Bond, Terms, ADJUSTMENT, BOND_INFO, CONFIG, TERMS},
    ContractError,
};
use staking_contract::msg::ConfigResponse;

pub fn total_base_supply(deps: Deps) -> Result<Uint128, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let staking_config: ConfigResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(
        cosmwasm_std::WasmQuery::Smart {
            contract_addr: config.staking.to_string(),
            msg: to_json_binary(&staking_contract::msg::QueryMsg::Config {})?,
        },
    ))?;
    let supply: SupplyResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Bank(
        cosmwasm_std::BankQuery::Supply {
            denom: staking_config.ohm_denom,
        },
    ))?;
    Ok(supply.amount.amount)
}

pub fn debt_ratio(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    let base_supply = total_base_supply(deps)?;

    Ok(Decimal256::from_ratio(
        current_debt(deps, env)?,
        base_supply,
    ))
}

pub fn circulating_supply(deps: Deps) -> Result<Uint128, ContractError> {
    Ok(cw20_base::contract::query_token_info(deps)?.total_supply)
}

pub fn query_config(deps: Deps) -> Result<crate::msg::ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(crate::msg::ConfigResponse {
        principle: config.principle,
        admin: config.admin.to_string(),
        staking: config.staking.to_string(),
        treasury: config.treasury.to_string(),
    })
}

pub fn query_terms(deps: Deps) -> Result<Terms, ContractError> {
    Ok(TERMS.load(deps.storage)?)
}

pub fn query_adjustment(deps: Deps) -> Result<Adjustment, ContractError> {
    Ok(ADJUSTMENT.load(deps.storage)?)
}

pub fn payout_for(deps: Deps, env: Env, value: Uint128) -> Result<Uint128, ContractError> {
    let payout = Decimal256::from_ratio(value, 1u128) / query_bond_price(deps, env)?;

    Ok((payout * Uint256::one()).try_into()?)
}

pub fn pending_payout_for(
    deps: Deps,
    env: Env,
    recipient: String,
) -> Result<Uint128, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let percent_vested = percent_vested_for(deps, env, &recipient_addr)?;
    let bond = BOND_INFO
        .load(deps.storage, &recipient_addr)
        .unwrap_or_default();

    if percent_vested > Decimal256::one() {
        Ok(bond.payout)
    } else {
        Ok((Uint256::from(bond.payout) * percent_vested).try_into()?)
    }
}

pub fn max_payout(deps: Deps) -> Result<Uint128, ContractError> {
    let base_supply = total_base_supply(deps)?;
    let terms = TERMS.load(deps.storage)?;

    Ok((Uint256::from(base_supply) * terms.max_payout).try_into()?)
}

pub fn percent_vested_for(
    deps: Deps,
    env: Env,
    recipient: &Addr,
) -> Result<Decimal256, ContractError> {
    let bond = BOND_INFO.load(deps.storage, recipient)?;
    // Bond memory bond = bondInfo[ _depositor ];
    let seconds_since_last = env.block.time.seconds() - bond.last_time.seconds();
    let vesting = bond.vesting_time_left;
    if vesting != 0 {
        Ok(Decimal256::from_ratio(seconds_since_last, vesting))
    } else {
        Ok(Decimal256::zero())
    }
}

pub fn bond_info(deps: Deps, recipient: String) -> Result<Bond, ContractError> {
    Ok(BOND_INFO
        .load(deps.storage, &deps.api.addr_validate(&recipient)?)
        .unwrap_or_default())
}
