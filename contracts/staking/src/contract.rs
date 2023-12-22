#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, Timestamp,
};

use crate::error::ContractError;
use crate::execute::{mint, rebase, stake, unstake};
use crate::helpers::{create_denom_msg, mint_msgs};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{base_denom, query_config, query_exchange_rate};
use crate::state::{
    Config, EpochState, MinterInfo, BASE_TOKEN_DENOM, CONFIG, EPOCH_STATE, MINTER_INFO,
    STAKING_TOKEN_DENOM,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        admin: msg
            .admin
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .unwrap_or(info.sender),
        epoch_length: msg.epoch_length,
        epoch_apr: msg.epoch_apr,
    };

    let state = EpochState {
        epoch_end: Timestamp::from_seconds(msg.first_epoch_time),
        epoch_number: 0,
    };
    CONFIG.save(deps.storage, &config)?;
    EPOCH_STATE.save(deps.storage, &state)?;

    // We create the base and the staked currency denomination
    // Don't forget to send some funds to the contract to create a denomination
    let base_currency_msg = create_denom_msg(&env, BASE_TOKEN_DENOM.to_string());

    let base_mint_msgs = msg
        .initial_balances
        .iter()
        .flat_map(|(receiver, balance)| {
            mint_msgs(&env, base_denom(&env), receiver.clone(), *balance)
        })
        .collect::<Vec<_>>();

    let staked_currency_msg = create_denom_msg(&env, STAKING_TOKEN_DENOM.to_string());

    Ok(Response::new()
        .add_message(base_currency_msg)
        .add_messages(base_mint_msgs)
        .add_message(staked_currency_msg))
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
        ExecuteMsg::Stake { to } => stake(deps, env, info, to),
        ExecuteMsg::Unstake { to } => unstake(deps, env, info, to),
        ExecuteMsg::Rebase {} => rebase(deps, env, info),
        ExecuteMsg::Mint { to, amount } => mint(deps, env, info, to, amount),
        ExecuteMsg::UpdateConfig {
            admin,
            epoch_length,
            epoch_apr,
            add_minter,
            remove_minter,
        } => update_config(
            deps,
            info,
            admin,
            epoch_length,
            epoch_apr,
            add_minter,
            remove_minter,
        ),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::ExchangeRate {} => Ok(to_json_binary(&query_exchange_rate(deps, env)?)?),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    epoch_length: Option<u64>,
    epoch_apr: Option<Decimal256>,
    add_minter: Option<Vec<String>>,
    remove_minter: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }
    if let Some(epoch_length) = epoch_length {
        config.epoch_length = epoch_length;
    }
    if let Some(epoch_apr) = epoch_apr {
        config.epoch_apr = epoch_apr;
    }
    if let Some(add_minter) = add_minter {
        for minter in add_minter {
            MINTER_INFO.save(
                deps.storage,
                &deps.api.addr_validate(&minter)?,
                &MinterInfo { can_mint: true },
            )?;
        }
    }
    if let Some(remove_minter) = remove_minter {
        for minter in remove_minter {
            MINTER_INFO.save(
                deps.storage,
                &deps.api.addr_validate(&minter)?,
                &MinterInfo { can_mint: false },
            )?;
        }
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

#[cfg(test)]
pub mod test {
    use cosmwasm_std::{coins, Decimal256};
    use cw_orch::{injective_test_tube::InjectiveTestTube, prelude::*};
    use std::str::FromStr;

    use cw_orch::injective_test_tube::injective_test_tube::Account;
    use staking::interface::Staking;
    use staking::msg::ExecuteMsgFns;
    use staking::msg::InstantiateMsg;
    use staking::msg::QueryMsgFns;
    use tests::tokenfactory::assert_balance;
    pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;
    pub const FUNDS_MULTIPLIER: u128 = 100_000;
    pub const EPOCH_LENGTH: u64 = 100;

    pub fn init() -> anyhow::Result<Staking<InjectiveTestTube>> {
        let chain = InjectiveTestTube::new(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"));

        let contract = Staking::new("staking", chain.clone());
        contract.upload()?;

        let block_info = chain.block_info()?;

        contract.instantiate(
            &InstantiateMsg {
                admin: None,
                epoch_apr: Decimal256::from_str("0.1")?,
                first_epoch_time: block_info.time.seconds() + 1,
                epoch_length: EPOCH_LENGTH,
                initial_balances: vec![(chain.sender().to_string(), 1_000_000u128.into())],
            },
            None,
            Some(&coins(AMOUNT_TO_CREATE_DENOM * 2, "inj")),
        )?;

        Ok(contract)
    }

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        let contract = init()?;
        let chain = contract.get_chain().clone();
        assert_balance(
            chain.clone(),
            contract.config()?.ohm,
            1_000_000,
            chain.sender().to_string(),
        )?;

        Ok(())
    }

    #[test]
    pub fn stake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            contract.address()?.to_string(),
        )?;
        assert_balance(chain, sohm_denom, 10_000, receiver.address().to_string())?;

        Ok(())
    }

    #[test]
    pub fn unstake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let chain = contract.get_chain().clone();
        let sender = chain.sender();

        contract.stake(sender.to_string(), &coins(10_000, contract.config()?.ohm))?;

        let sohm_denom = contract.config()?.sohm;

        contract.unstake(sender.to_string(), &coins(10_000, sohm_denom.clone()))?;

        assert_balance(chain.clone(), sohm_denom, 0, sender.to_string())?;
        assert_balance(
            chain.clone(),
            contract.config()?.ohm,
            1_000_000,
            sender.to_string(),
        )?;
        Ok(())
    }

    #[test]
    pub fn stake_with_different_exchange_rates_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            10_000,
            receiver.address().to_string(),
        )?;

        // We send some tokens to the contract, this should double the exchange rate
        chain.bank_send(
            contract.address()?.to_string(),
            coins(10_000, contract.config()?.ohm),
        )?;

        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom,
            15_000,
            receiver.address().to_string(),
        )?;

        Ok(())
    }

    #[test]
    pub fn stake_with_weird_exchange_rates_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            10_000,
            receiver.address().to_string(),
        )?;

        // We send some tokens to the contract, this should double the exchange rate
        chain.bank_send(
            contract.address()?.to_string(),
            coins(2_563, contract.config()?.ohm),
        )?;

        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom,
            10_000 + 10_000 * 10_000 / (10_000 + 2_563),
            receiver.address().to_string(),
        )?;

        Ok(())
    }

    #[test]
    pub fn two_people_stake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        let ohm_denom = contract.config()?.ohm;
        chain.bank_send(
            receiver.address().to_string(),
            coins(5_000, ohm_denom.clone()),
        )?;

        contract.stake(
            chain.sender().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        contract.call_as(&receiver).stake(
            receiver.address().to_string(),
            &coins(5_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            5_000,
            receiver.address().to_string(),
        )?;
        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            10_000,
            chain.sender().to_string(),
        )?;
        assert_balance(chain, ohm_denom, 0, receiver.address().to_string())?;

        Ok(())
    }

    #[test]
    pub fn two_people_stake_withdraw_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        let ohm_denom = contract.config()?.ohm;
        chain.bank_send(
            receiver.address().to_string(),
            coins(5_000, ohm_denom.clone()),
        )?;

        contract.stake(
            chain.sender().to_string(),
            &coins(10_000, contract.config()?.ohm),
        )?;

        contract.call_as(&receiver).stake(
            receiver.address().to_string(),
            &coins(5_000, contract.config()?.ohm),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            5_000,
            receiver.address().to_string(),
        )?;
        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            10_000,
            chain.sender().to_string(),
        )?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            0,
            receiver.address().to_string(),
        )?;

        contract.unstake(
            chain.sender().to_string(),
            &coins(10_000, sohm_denom.clone()),
        )?;

        contract.call_as(&receiver).unstake(
            receiver.address().to_string(),
            &coins(5_000, sohm_denom.clone()),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            receiver.address().to_string(),
        )?;
        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            chain.sender().to_string(),
        )?;
        assert_balance(chain, ohm_denom, 5_000, receiver.address().to_string())?;

        Ok(())
    }

    #[test]
    pub fn two_people_stake_rebase_withdraw_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_denom = contract.config()?.sohm;
        let ohm_denom = contract.config()?.ohm;
        chain.bank_send(
            receiver.address().to_string(),
            coins(500_000, ohm_denom.clone()),
        )?;

        contract.stake(
            chain.sender().to_string(),
            &coins(500_000, contract.config()?.ohm),
        )?;

        contract.call_as(&receiver).stake(
            receiver.address().to_string(),
            &coins(500_000, contract.config()?.ohm),
        )?;

        // We advance time to make sure we can rebase

        contract.rebase()?;

        contract.unstake(
            chain.sender().to_string(),
            &coins(500_000, sohm_denom.clone()),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            contract.address()?.to_string(),
        )?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            550_000,
            contract.address()?.to_string(),
        )?;

        chain.wait_blocks(EPOCH_LENGTH)?;
        contract.rebase()?;

        contract.call_as(&receiver).unstake(
            receiver.address().to_string(),
            &coins(500_000, sohm_denom.clone()),
        )?;
        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            contract.address()?.to_string(),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            0,
            receiver.address().to_string(),
        )?;
        assert_balance(chain.clone(), sohm_denom, 0, chain.sender().to_string())?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            550_000,
            chain.sender().to_string(),
        )?;
        assert_balance(chain, ohm_denom, 605_000, receiver.address().to_string())?;

        Ok(())
    }
}
