use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, Uint128,
};

use crate::{
    helpers::deposit_one_coin,
    query::{max_payout, payout_for},
    state::{bond_price, bond_price_in_usd, BOND_INFO, CONFIG, LAST_DECAY, TERMS, TOTAL_DEBT},
    ContractError,
};

pub const INITIAL_FRAGMENTS_SUPPLY: u128 = 5_000_000;
pub const TOTAL_GONS: u128 = u128::MAX - (u128::MAX % INITIAL_FRAGMENTS_SUPPLY);
pub const MAX_SUPPLY: u128 = u64::MAX as u128;

pub fn current_debt(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let total_debt = TOTAL_DEBT.load(deps.storage)?;
    Ok(total_debt - debt_decay(deps, env)?)
}

pub fn debt_decay(deps: Deps, env: Env) -> Result<Uint128, StdError> {
    let terms = TERMS.load(deps.storage)?;
    let last_decay = LAST_DECAY.load(deps.storage)?;
    let time_since_last_decay = env.block.time.seconds() - last_decay.seconds();
    let total_debt = TOTAL_DEBT.load(deps.storage)?;

    let mut decay =
        total_debt * Uint128::from(time_since_last_decay) / Uint128::from(terms.vesting_term);
    if decay > total_debt {
        decay = total_debt;
    }
    Ok(decay)
}

pub fn decay_debt(deps: DepsMut, env: Env, info: MessageInfo) -> Result<(), ContractError> {
    TOTAL_DEBT.update(deps.storage, |d| {
        Ok::<_, ContractError>(d - debt_decay(deps.as_ref(), env)?)
    })?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;

    Ok(())
}

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_price: Decimal256,
    depositor: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let deposited_amount = deposit_one_coin(info, config.principle)?;

    decay_debt(deps, env, info);
    let current_debt = TOTAL_DEBT.load(deps.storage)?;
    let terms = TERMS.load(deps.storage)?;

    ensure!(
        current_debt <= terms.max_debt,
        StdError::generic_err("Max capacity reached")
    );

    let price_in_usd = bond_price_in_usd(deps.as_ref(), env)?;
    let native_price = bond_price(deps, env)?;

    ensure!(
        max_price >= native_price,
        StdError::generic_err("Slippage limit: more than max price")
    );
    let payout = payout_for(deps.as_ref(), env, deposited_amount)?;

    ensure!(
        payout.u128() >= 1_000,
        StdError::generic_err("Bond too small")
    ); // must be > 0.001 OHM ( underflow protection )
    ensure!(
        payout <= max_payout(deps.as_ref())?,
        StdError::generic_err("Bond too large")
    ); // size protection because there is no slippage

    let treasury_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.treasury.to_string(),
        amount: vec![Coin {
            amount: deposited_amount,
            denom: config.principle.clone(),
        }],
    });

    let mint_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.staking.to_string(),
        msg: to_json_binary(&staking::msg::ExecuteMsg::Mint {
            to: env.contract.address.to_string(),
            amount: payout,
        })?,
        funds: vec![],
    });

    TOTAL_DEBT.update(deps.storage, |d| Ok::<_, ContractError>(d + payout))?;

    let depositor_addr = deps.api.addr_validate(&depositor)?;
    BOND_INFO.update(deps.storage, &depositor_addr, |b| {
        let mut bond = b.unwrap_or_default();

        bond.payout += payout;
        bond.vesting_time_left = terms.vesting_term;
        bond.last_time = env.block.time;
        bond.price_paid = price_in_usd;

        Ok::<_, ContractError>(bond)
    })?;
    Ok(Response::new()
        .add_message(treasury_msg)
        .add_message(mint_msg))
}
