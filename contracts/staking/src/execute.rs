use cosmos_sdk_proto::traits::Message;
use cosmwasm_std::{
    coins, to_json_binary, BankMsg, Coin, CosmosMsg, Decimal256, DepsMut, Empty, Env, MessageInfo,
    Response, StdError, SubMsg, Uint128, Uint256,
};
use cw20::MinterResponse;
use injective_std::types::injective::tokenfactory::v1beta1::MsgMint;

use crate::{
    contract::{INSTANTIATE_ADMIN_CONTRACT_REPLY, INSTANTIATE_STAKING_TOKEN_REPLY},
    helpers::{deposit_one_coin, mint_msgs},
    query::{base_denom, current_exchange_rate, staking_token_addr, token_balance},
    state::{
        update_staking_points, StakingPoints, Warmup, BOND_CONTRACT_INFO, CONFIG, EPOCH_STATE,
        STAKING_POINTS, WARMUP,
    },
    ContractError,
};

pub const INITIAL_FRAGMENTS_SUPPLY: u128 = 5_000_000;
pub const TOTAL_GONS: u128 = u128::MAX - (u128::MAX % INITIAL_FRAGMENTS_SUPPLY);
pub const MAX_SUPPLY: u128 = u64::MAX as u128;

pub fn rebase(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let mut epoch_state = EPOCH_STATE.load(deps.storage)?;

    if epoch_state.epoch_end > env.block.time {
        return Ok(Response::new());
    }

    // We mint some new OHM
    let current_balance = token_balance(deps.as_ref(), &env)?;
    let rebase_amount = Uint256::from(current_balance) * config.epoch_apr;

    // Mint some new ohm to this contract : this is where the APR comes from !
    let msg = if rebase_amount.is_zero() {
        None
    } else {
        Some(CosmosMsg::Stargate {
            type_url: MsgMint::TYPE_URL.to_string(),
            value: MsgMint {
                sender: env.contract.address.to_string(),
                amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
                    denom: base_denom(&env),
                    amount: rebase_amount.to_string(),
                }),
            }
            .encode_to_vec()
            .into(),
        })
    };

    epoch_state.epoch_start = epoch_state.epoch_end;
    epoch_state.epoch_end = epoch_state.epoch_end.plus_seconds(config.epoch_length);
    epoch_state.epoch_number += 1;

    if let Some(next_epoch_apr) = config.next_epoch_apr {
        config.next_epoch_apr = None;
        config.epoch_apr = next_epoch_apr;
    }

    EPOCH_STATE.save(deps.storage, &epoch_state)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_messages(msg))
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let deposited_amount = deposit_one_coin(info, base_denom(&env))?;
    let config = CONFIG.load(deps.storage)?;

    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, Some(deposited_amount))?;
    let mint_amount =
        (Decimal256::from_ratio(deposited_amount, 1u128) / exchange_rate) * Uint256::one();

    let to_addr = deps.api.addr_validate(&to)?;
    // Add to current user Warmup
    WARMUP.update(deps.storage, &to_addr, |w| match w {
        None => Ok::<_, ContractError>(Warmup {
            amount: deposited_amount,
            mint_amount: mint_amount.try_into()?,
            end: env.block.time.plus_seconds(config.warmup_length),
        }),
        Some(mut w) => {
            w.amount += deposited_amount;
            w.mint_amount += Uint128::try_from(mint_amount)?;
            w.end = env.block.time.plus_seconds(config.warmup_length);
            Ok(w)
        }
    })?;

    // We send the deposited_asset into the warmup contract
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.warmup_address.unwrap().to_string(),
        amount: vec![Coin {
            amount: deposited_amount,
            denom: base_denom(&env),
        }],
    });

    Ok(Response::new().add_message(msg))
}

pub fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // We transfer coins from the warmup contract to us
    let claim = WARMUP.load(deps.storage, &info.sender)?;
    WARMUP.remove(deps.storage, &info.sender);

    let msgs = if claim.end > env.block.time {
        // We send the ohm back to the sender
        vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: config.warmup_address.unwrap().to_string(),
            msg: to_json_binary(&cw1_whitelist::msg::ExecuteMsg::Execute {
                msgs: vec![CosmosMsg::Bank::<Empty>(BankMsg::Send {
                    to_address: to,
                    amount: coins(claim.amount.u128(), base_denom(&env)),
                })],
            })?,
            funds: vec![],
        })]
    } else {
        let to_addr = deps.api.addr_validate(&to)?;
        update_staking_points(deps.branch(), env.clone(), &to_addr, claim.mint_amount)?;
        // We mint some sOHM to the to address
        vec![
            CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.warmup_address.unwrap().to_string(),
                msg: to_json_binary(&cw1_whitelist::msg::ExecuteMsg::Execute {
                    msgs: vec![CosmosMsg::Bank::<Empty>(BankMsg::Send {
                        to_address: env.contract.address.to_string(),
                        amount: coins(claim.amount.u128(), base_denom(&env)),
                    })],
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: staking_token_addr(deps.as_ref())?.to_string(),
                msg: to_json_binary(&cw20_base::msg::ExecuteMsg::Mint {
                    recipient: to,
                    amount: claim.mint_amount,
                })?,
                funds: vec![],
            }),
        ]
    };

    Ok(Response::new().add_messages(msgs))
}

pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, None)?;

    let redeem_amount = Uint256::from(amount) * exchange_rate;

    // We update the staking points
    STAKING_POINTS.update(deps.storage, &info.sender, |points| match points {
        None => Ok::<_, StdError>(StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: env.block.time,
        }),
        Some(mut staking_points) => {
            staking_points.last_points_updated = env.block.time;
            staking_points.total_points = Uint128::zero();
            Ok(staking_points)
        }
    })?;

    // We burn the received sOHM from this contract
    let burn_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
        contract_addr: staking_token_addr(deps.as_ref())?.to_string(),
        msg: to_json_binary(&cw20_base::msg::ExecuteMsg::BurnFrom {
            owner: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    });

    // We send OHM back to the depositor
    let send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: to,
        amount: vec![Coin {
            amount: redeem_amount.try_into()?,
            denom: base_denom(&env),
        }],
    });

    Ok(Response::new().add_message(burn_msg).add_message(send_msg))
}

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let minter_info =
        BOND_CONTRACT_INFO
            .load(deps.storage, &info.sender)
            .or(Err(StdError::generic_err(format!(
                "{} is not authorized to mint on staking",
                info.sender
            ))))?;
    if !minter_info.can_mint {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new().add_messages(mint_msgs(&env, base_denom(&env), to, amount)))
}

pub fn instantiate_staking_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staking_token_code_id: u64,
    staking_symbol: String,
    staking_name: String,
    cw1_code_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    if config.staking_denom_address.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    let staked_currency_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Instantiate {
            admin: Some(env.contract.address.to_string()),
            code_id: staking_token_code_id,
            msg: to_json_binary(&cw20_base::msg::InstantiateMsg {
                name: staking_name,
                symbol: staking_symbol,
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: "Staking token".to_string(),
        }),
        INSTANTIATE_STAKING_TOKEN_REPLY,
    );

    let cw1_instantiate_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Instantiate {
            admin: Some(env.contract.address.to_string()),
            code_id: cw1_code_id,
            msg: to_json_binary(&cw1_whitelist::msg::InstantiateMsg {
                admins: vec![env.contract.address.to_string()],
                mutable: false,
            })?,
            funds: vec![],
            label: "Warmup contract".to_string(),
        }),
        INSTANTIATE_ADMIN_CONTRACT_REPLY,
    );

    Ok(Response::new()
        .add_submessage(staked_currency_msg)
        .add_submessage(cw1_instantiate_msg))
}
