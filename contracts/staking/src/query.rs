use cosmwasm_std::{to_json_binary, Addr, Decimal256, Deps, Env, StdError, StdResult, Uint128};
use cw20::TokenInfoResponse;

use crate::{
    msg::ConfigResponse,
    state::{
        staking_points_update_closure, StakingPoints, BASE_TOKEN_DENOM, CONFIG, EPOCH_STATE,
        STAKING_POINTS,
    },
    ContractError,
};

pub fn base_denom(env: &Env) -> String {
    factory_denom(env, BASE_TOKEN_DENOM)
}
// pub fn staking_denom(env: &Env) -> String {
//     factory_denom(env, STAKING_TOKEN_DENOM)
// }
pub fn staking_token_addr(deps: Deps) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config
        .staking_denom_address
        .expect("Has to be registered in reply"))
}

pub fn factory_denom(env: &Env, subdenom: impl ToString) -> String {
    format!("factory/{}/{}", env.contract.address, subdenom.to_string())
}

pub fn token_balance(deps: Deps, env: &Env) -> Result<Uint128, StdError> {
    let balance: cosmwasm_std::BalanceResponse = deps.querier.query(
        &cosmwasm_std::QueryRequest::Bank(cosmwasm_std::BankQuery::Balance {
            address: env.contract.address.to_string(),
            denom: base_denom(env),
        }),
    )?;

    Ok(balance.amount.amount)
}
pub fn staking_token_balance(deps: Deps, address: &Addr) -> Result<Uint128, StdError> {
    let balance: cw20::BalanceResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(
        cosmwasm_std::WasmQuery::Smart {
            contract_addr: staking_token_addr(deps)?.to_string(),
            msg: to_json_binary(&cw20_base::msg::QueryMsg::Balance {
                address: address.to_string(),
            })?,
        },
    ))?;

    Ok(balance.balance)
}

/// This represents the value of each staking token compared to the base token
/// For instance, if this contracts hold 100 CW20 and has minted 80 sCW20, the exchange rate is 100/80 = 1.25
pub fn current_exchange_rate(
    deps: Deps,
    env: &Env,
    deposit: Option<Uint128>,
) -> Result<Decimal256, ContractError> {
    let deposited_amount = token_balance(deps, env)? - deposit.unwrap_or(Uint128::zero());

    let minted_staked_currency: TokenInfoResponse = deps.querier.query(
        &cosmwasm_std::QueryRequest::Wasm(cosmwasm_std::WasmQuery::Smart {
            contract_addr: staking_token_addr(deps)?.to_string(),
            msg: to_json_binary(&cw20_base::msg::QueryMsg::TokenInfo {})?,
        }),
    )?;

    let staked_amount = minted_staked_currency.total_supply;

    if staked_amount == Uint128::zero() || deposited_amount <= staked_amount {
        Ok(Decimal256::one())
    } else {
        Ok(Decimal256::from_ratio(deposited_amount, staked_amount))
    }
}

pub fn expected_exchange_rate(
    deps: Deps,
    env: &Env,
    deposit: Option<Uint128>,
) -> Result<Decimal256, ContractError> {
    let current_exchange_rate = current_exchange_rate(deps, env, deposit)?;
    let config = CONFIG.load(deps.storage)?;
    let epoch_state = EPOCH_STATE.load(deps.storage)?;

    let blocks_since_epoch = if env.block.time.seconds() >= epoch_state.epoch_start.seconds() {
        env.block.time.seconds() - epoch_state.epoch_start.seconds()
    } else {
        0u64
    };

    let lost_exchange_rate =
        config.epoch_apr * Decimal256::from_ratio(blocks_since_epoch, config.epoch_length);

    Ok(current_exchange_rate * (Decimal256::one() + lost_exchange_rate))
}

pub fn query_config(deps: Deps, env: Env) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        epoch_length: config.epoch_length,
        epoch_apr: config.epoch_apr,
        next_epoch_apr: config.next_epoch_apr,
        admin: config.admin.to_string(),
        ohm_denom: base_denom(&env),
        sohm_address: staking_token_addr(deps)?.to_string(),
    })
}

pub fn query_exchange_rate(deps: Deps, env: Env) -> Result<Decimal256, ContractError> {
    current_exchange_rate(deps, &env, None)
}

pub fn query_raw_staking_points(
    deps: Deps,
    address: String,
) -> Result<StakingPoints, ContractError> {
    let address_addr = deps.api.addr_validate(&address)?;
    Ok(STAKING_POINTS.load(deps.storage, &address_addr)?)
}

pub fn query_current_staking_points(
    deps: Deps,
    env: Env,
    address: String,
) -> Result<StakingPoints, ContractError> {
    let address_addr = deps.api.addr_validate(&address)?;
    let raw_staking_points = query_raw_staking_points(deps, address).ok();
    let current_stake = staking_token_balance(deps, &address_addr)?;

    staking_points_update_closure(&env, current_stake, raw_staking_points)
}
