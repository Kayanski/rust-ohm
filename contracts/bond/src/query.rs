use cosmwasm_std::{
    to_json_binary, Decimal256, Deps, Env, StdError, SupplyResponse, Timestamp, Uint128, Uint256,
};
use oracle::msg::PriceResponse;

use crate::{
    execute::current_debt,
    state::{query_bond_price, Config, CONFIG, TERMS},
    ContractError,
};
use staking::msg::ConfigResponse;

pub fn total_base_supply(deps: Deps) -> Result<Uint128, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let staking_config: ConfigResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(
        cosmwasm_std::WasmQuery::Smart {
            contract_addr: config.staking.to_string(),
            msg: to_json_binary(&staking::msg::QueryMsg::Config {})?,
        },
    ))?;
    let supply: SupplyResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Bank(
        cosmwasm_std::BankQuery::Supply {
            denom: staking_config.ohm,
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

pub fn query_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

pub fn asset_price(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // We query the price from the oracle
    let price: PriceResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(
        cosmwasm_std::WasmQuery::Smart {
            contract_addr: config.oracle.to_string(),
            msg: to_json_binary(&oracle::msg::QueryMsg::Price {
                base: config.usd,
                quote: config.principle,
            })?,
        },
    ))?;

    // We assert the price is not too old
    if Timestamp::from_seconds(price.last_updated_base).plus_seconds(config.oracle_trust_period)
        < env.block.time
        || Timestamp::from_seconds(price.last_updated_quote)
            .plus_seconds(config.oracle_trust_period)
            < env.block.time
    {
        Err(StdError::generic_err("Price data is too old for bonding"))?;
    }
    Ok(price.rate)
}

pub fn payout_for(deps: Deps, env: Env, value: Uint128) -> Result<Uint128, ContractError> {
    let payout = Decimal256::new(value.into()) / query_bond_price(deps, env)?;

    Ok((payout * Uint256::one()).try_into()?)
}

pub fn max_payout(deps: Deps) -> Result<Uint128, ContractError> {
    let base_supply = total_base_supply(deps)?;
    let terms = TERMS.load(deps.storage)?;

    Ok((Uint256::from(base_supply) * terms.max_payout).try_into()?)
}
