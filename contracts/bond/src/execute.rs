use cosmwasm_std::{
    ensure, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, Uint128, Uint256,
};
use staking_contract::msg::ConfigResponse;

use crate::{
    helpers::{adjust, deposit_one_coin},
    query::{max_payout, payout_for, percent_vested_for},
    state::{bond_price, BOND_INFO, CONFIG, LAST_DECAY, TERMS, TOTAL_DEBT},
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

pub fn decay_debt(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<(), ContractError> {
    let debt_decay = debt_decay(deps.as_ref(), env.clone())?;
    TOTAL_DEBT.update(deps.storage, |d| Ok::<_, ContractError>(d - debt_decay))?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;

    Ok(())
}

pub fn deposit(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_price: Decimal256,
    depositor: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let deposited_amount = deposit_one_coin(info.clone(), config.principle.clone())?;

    decay_debt(deps.branch(), env.clone(), info)?;
    let total_debt = TOTAL_DEBT.load(deps.storage)?;
    let terms = TERMS.load(deps.storage)?;

    ensure!(
        total_debt <= terms.max_debt,
        StdError::generic_err("Max capacity reached")
    );

    let native_price = bond_price(deps.branch(), env.clone())?;

    ensure!(
        max_price >= native_price,
        StdError::generic_err("Slippage limit: more than max price")
    );
    let payout = payout_for(deps.as_ref(), env.clone(), deposited_amount)?;

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
        msg: to_json_binary(&staking_contract::msg::ExecuteMsg::Mint {
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
        bond.last_time = env.clone().block.time;

        Ok::<_, ContractError>(bond)
    })?;

    adjust(deps, env)?;
    Ok(Response::new()
        .add_message(treasury_msg)
        .add_message(mint_msg))
}

pub fn redeem(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    recipient: String,
    stake: bool,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let mut bond = BOND_INFO.load(deps.storage, &recipient_addr)?;
    let percent_vested = percent_vested_for(deps.as_ref(), env.clone(), &recipient_addr)?;
    if percent_vested >= Decimal256::one() {
        BOND_INFO.remove(deps.storage, &recipient_addr);
        return stake_or_send(deps.as_ref(), recipient_addr, stake, bond.payout);
    }
    let payout = Uint256::from(bond.payout) * percent_vested;
    if payout.is_zero() {
        Err(StdError::generic_err("Nothing to redeem here !"))?;
    }

    bond.payout -= Uint128::try_from(payout)?;
    bond.vesting_time_left -= env.block.time.seconds() - bond.last_time.seconds();
    bond.last_time = env.block.time;

    BOND_INFO.save(deps.storage, &recipient_addr, &bond)?;

    stake_or_send(deps.as_ref(), recipient_addr, stake, payout.try_into()?)
}

pub fn stake_or_send(
    deps: Deps,
    recipient: Addr,
    stake: bool,
    payout: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let staking_config: ConfigResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(
        cosmwasm_std::WasmQuery::Smart {
            contract_addr: config.staking.to_string(),
            msg: to_json_binary(&staking_contract::msg::QueryMsg::Config {})?,
        },
    ))?;
    let payout_coins = vec![Coin {
        amount: payout,
        denom: staking_config.ohm_denom,
    }];

    let msgs = if !stake {
        CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: payout_coins,
        })
    } else {
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: config.staking.to_string(),
            msg: to_json_binary(&staking_contract::msg::ExecuteMsg::Stake {
                to: recipient.to_string(),
            })?,
            funds: payout_coins,
        })
    };
    Ok(Response::new().add_message(msgs))
}
