use cosmwasm_std::{Decimal256, Deps, Env, StdError, SupplyResponse, Uint128};

use crate::{
    state::{Config, CONFIG, STAKING_TOKEN_DENOM},
    ContractError,
};

pub fn staking_denom(env: &Env) -> String {
    format!("factory/{}/{}", env.contract.address, STAKING_TOKEN_DENOM)
}

pub fn circulating_supply(deps: Deps) -> Result<Uint128, ContractError> {
    Ok(cw20_base::contract::query_token_info(deps)?.total_supply)
}

pub fn token_balance(deps: Deps, env: &Env) -> Result<Uint128, StdError> {
    let config = CONFIG.load(deps.storage)?;
    let balance: cosmwasm_std::BalanceResponse = deps.querier.query(
        &cosmwasm_std::QueryRequest::Bank(cosmwasm_std::BankQuery::Balance {
            address: env.contract.address.to_string(),
            denom: config.ohm,
        }),
    )?;

    Ok(balance.amount.amount)
}

/// This represents the value of each staking token compared to the base token
/// For instance, if this contracts hold 100 CW20 and has minted 80 sCW20, the exchange rate is 100/80 = 1.25
pub fn current_exchange_rate(
    deps: Deps,
    env: &Env,
    deposit: Option<Uint128>,
    deposit_staked: Option<Uint128>,
) -> Result<Decimal256, ContractError> {
    let deposited_amount = token_balance(deps, env)? - deposit.unwrap_or(Uint128::zero());

    let minted_staked_currency: SupplyResponse = deps.querier.query(
        &cosmwasm_std::QueryRequest::Bank(cosmwasm_std::BankQuery::Supply {
            denom: staking_denom(env),
        }),
    )?;

    let staked_amount = minted_staked_currency.amount.amount - deposit_staked.unwrap_or_default();

    if staked_amount == Uint128::zero() || deposited_amount <= staked_amount {
        Ok(Decimal256::one())
    } else {
        Ok(Decimal256::from_ratio(deposited_amount, staked_amount))
    }
}

pub fn query_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

pub fn query_exchange_rate(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    current_exchange_rate(deps, &env, None, None)
}
