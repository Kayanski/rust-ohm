use crate::response::MsgInstantiateContractResponse;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, Timestamp,
};
use protobuf::Message;

use crate::error::ContractError;
use crate::execute::{
    execute_claim, execute_stake, instantiate_staking_token, mint, rebase, unstake,
};
use crate::helpers::{create_denom_msg, mint_msgs};
use crate::msg::{BondContractsElem, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{
    base_denom, query_config, query_current_staking_points, query_exchange_rate,
    query_raw_staking_points,
};
use crate::state::{
    bond_contracts, BondContractInfo, Config, EpochState, BASE_TOKEN_DENOM, BOND_CONTRACT_INFO,
    CONFIG, EPOCH_STATE,
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
        staking_denom_address: None,
        next_epoch_apr: None,
        warmup_address: None,
        warmup_length: msg.warmup_length,
    };

    let state = EpochState {
        epoch_start: Timestamp::from_seconds(msg.first_epoch_time),
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

    Ok(Response::new()
        .add_message(base_currency_msg)
        .add_messages(base_mint_msgs))
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
        ExecuteMsg::Stake { to } => execute_stake(deps, env, info, to),
        ExecuteMsg::Claim { to } => execute_claim(deps, env, info, to),
        ExecuteMsg::Unstake { to, amount } => unstake(deps, env, info, to, amount),
        ExecuteMsg::Rebase {} => rebase(deps, env, info),
        ExecuteMsg::Mint { to, amount } => mint(deps, env, info, to, amount),
        ExecuteMsg::UpdateConfig {
            admin,
            epoch_length,
            epoch_apr,
            add_bond,
            remove_bond,
        } => update_config(
            deps,
            info,
            admin,
            epoch_length,
            epoch_apr,
            add_bond,
            remove_bond,
        ),
        ExecuteMsg::InstantiateContracts {
            staking_token_code_id,
            staking_symbol,
            staking_name,
            cw1_code_id,
        } => instantiate_staking_token(
            deps,
            env,
            info,
            staking_token_code_id,
            staking_symbol,
            staking_name,
            cw1_code_id,
        ),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::ExchangeRate {} => Ok(to_json_binary(&query_exchange_rate(deps, env)?)?),
        QueryMsg::Bonds {} => Ok(to_json_binary(&bond_contracts(deps)?)?),
        QueryMsg::EpochState {} => Ok(to_json_binary(&EPOCH_STATE.load(deps.storage)?)?),
        QueryMsg::StakingPoints { address } => Ok(to_json_binary(&query_current_staking_points(
            deps, env, address,
        )?)?),
        QueryMsg::RawStakingPoints { address } => {
            Ok(to_json_binary(&query_raw_staking_points(deps, address)?)?)
        }
    }
}
pub const INSTANTIATE_STAKING_TOKEN_REPLY: u64 = 1;
pub const INSTANTIATE_ADMIN_CONTRACT_REPLY: u64 = 2;
/// Handling contract replies
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        INSTANTIATE_STAKING_TOKEN_REPLY => {
            // We register the instantiated contract in the config
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                reply.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                ContractError::Std(StdError::parse_err(
                    "MsgInstantiateContractResponse",
                    "failed to parse data",
                ))
            })?;
            let token_addr = Addr::unchecked(res.get_contract_address());

            CONFIG.update(deps.storage, |mut c| {
                c.staking_denom_address = Some(token_addr.clone());
                Ok::<_, StdError>(c)
            })?;
            Ok(Response::new().add_attributes(vec![attr("staking-token", token_addr)]))
        }
        INSTANTIATE_ADMIN_CONTRACT_REPLY => {
            // We register the instantiated contract in the config
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                reply.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                ContractError::Std(StdError::parse_err(
                    "MsgInstantiateContractResponse",
                    "failed to parse data",
                ))
            })?;
            let warmup_addr = Addr::unchecked(res.get_contract_address());

            CONFIG.update(deps.storage, |mut c| {
                c.warmup_address = Some(warmup_addr.clone());
                Ok::<_, StdError>(c)
            })?;
            Ok(Response::new().add_attributes(vec![attr("warmup", warmup_addr)]))
        }
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    epoch_length: Option<u64>,
    epoch_apr: Option<Decimal256>,
    add_bond: Option<Vec<BondContractsElem>>,
    remove_bond: Option<Vec<String>>,
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
        config.next_epoch_apr = Some(epoch_apr);
    }
    if let Some(add_bond) = add_bond {
        for bond in add_bond {
            BOND_CONTRACT_INFO.save(
                deps.storage,
                &deps.api.addr_validate(&bond.bond_address)?,
                &BondContractInfo {
                    can_mint: true,
                    bond_token: bond.bond_token,
                },
            )?;
        }
    }
    if let Some(remove_bond) = remove_bond {
        for bond in remove_bond {
            BOND_CONTRACT_INFO.remove(deps.storage, &deps.api.addr_validate(&bond)?);
        }
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

#[cfg(test)]
pub mod test {
    use cosmwasm_std::{coins, Decimal256};
    use cw20::msg::Cw20ExecuteMsgFns;
    use cw_orch::{injective_test_tube::InjectiveTestTube, prelude::*};
    use staking_token::interface::StakingToken;
    use std::str::FromStr;
    use tests::tokenfactory::assert_cw20_balance;

    use cw_orch::injective_test_tube::injective_test_tube::Account;
    use cw_plus_interface::cw1_whitelist::Cw1Whitelist;
    use staking_contract::interface::Staking;
    use staking_contract::msg::ExecuteMsgFns;
    use staking_contract::msg::InstantiateMsg;
    use staking_contract::msg::QueryMsgFns;
    use tests::tokenfactory::assert_balance;
    pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;
    pub const FUNDS_MULTIPLIER: u128 = 100_000;
    pub const EPOCH_LENGTH: u64 = 100;
    pub const WARMUP_LENGTH: u64 = 200;

    pub fn init() -> anyhow::Result<Staking<InjectiveTestTube>> {
        let chain = InjectiveTestTube::new(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"));

        let contract = Staking::new("staking", chain.clone());
        let token = StakingToken::new("staking-token", chain.clone());
        contract.upload()?;
        token.upload()?;

        let block_info = chain.block_info()?;

        let cw1 = Cw1Whitelist::new("cw1-whitelist", chain.clone());
        cw1.upload()?;

        contract.instantiate(
            &InstantiateMsg {
                admin: None,
                epoch_apr: Decimal256::from_str("0.1")?,
                first_epoch_time: block_info.time.seconds() + 1,
                epoch_length: EPOCH_LENGTH,
                initial_balances: vec![(chain.sender().to_string(), 1_000_000u128.into())],
                warmup_length: WARMUP_LENGTH,
            },
            None,
            Some(&coins(AMOUNT_TO_CREATE_DENOM * 2, "inj")),
        )?;

        contract.instantiate_contracts(
            cw1.code_id()?,
            "sSHOGUN".to_string(),
            "sSHGN".to_string(),
            token.code_id()?,
        )?;
        token.set_address(&Addr::unchecked(contract.config()?.sohm_address));

        Ok(contract)
    }

    pub fn token<Chain: CwEnv>(staking: &Staking<Chain>) -> anyhow::Result<StakingToken<Chain>> {
        let token = StakingToken::new("staking-token", staking.get_chain().clone());

        Ok(token)
    }

    pub fn unstake<Chain: CwEnv>(
        staking: &Staking<Chain>,
        amount: u128,
        receiver: Option<String>,
    ) -> anyhow::Result<()> {
        token(staking)?.increase_allowance(amount.into(), staking.address()?.to_string(), None)?;
        staking.unstake(
            amount.into(),
            receiver.unwrap_or_else(|| staking.get_chain().sender().to_string()),
            &[],
        )?;

        Ok(())
    }

    pub fn stake_and_claim<Chain: CwEnv>(
        staking: &Staking<Chain>,
        amount: u128,
        receiver: Option<String>,
    ) -> anyhow::Result<()> {
        let sender = staking.get_chain().sender().to_string();
        let receiver = receiver.unwrap_or_else(|| staking.get_chain().sender().to_string());
        staking.stake(sender, &coins(amount, staking.config()?.ohm_denom))?;
        staking.get_chain().wait_seconds(WARMUP_LENGTH).unwrap();

        staking.claim(receiver)?;

        Ok(())
    }

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        let contract = init()?;
        let chain = contract.get_chain().clone();
        assert_balance(
            chain.clone(),
            contract.config()?.ohm_denom,
            1_000_000,
            chain.sender().to_string(),
        )?;

        Ok(())
    }

    #[test]
    pub fn warmup_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_address = contract.config()?.sohm_address;
        contract.stake(
            receiver.address().to_string(),
            &coins(10_000, contract.config()?.ohm_denom),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            contract.address()?.to_string(),
        )?;
        assert_cw20_balance(
            chain.clone(),
            sohm_address,
            0,
            receiver.address().to_string(),
        )?;
        assert_balance(
            chain,
            contract.config()?.ohm_denom,
            10_000,
            contract.config()?.warmup_address,
        )?;

        contract.get_chain().wait_seconds(WARMUP_LENGTH).unwrap();
        contract
            .call_as(&receiver)
            .claim(receiver.address().to_string())?;
        contract
            .call_as(&receiver)
            .claim(receiver.address().to_string())
            .unwrap_err();

        Ok(())
    }

    #[test]
    pub fn stake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        let sohm_address = contract.config()?.sohm_address;
        stake_and_claim(&contract, 10_000, Some(receiver.address().to_string()))?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            contract.address()?.to_string(),
        )?;
        assert_cw20_balance(chain, sohm_address, 10_000, receiver.address().to_string())?;

        Ok(())
    }

    #[test]
    pub fn unstake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let chain = contract.get_chain().clone();
        let sender = chain.sender();

        stake_and_claim(&contract, 10_000, None)?;

        let sohm_address = contract.config()?.sohm_address;

        unstake(&contract, 10_000, None)?;

        assert_cw20_balance(chain.clone(), sohm_address, 0, sender.to_string())?;
        assert_balance(
            chain.clone(),
            contract.config()?.ohm_denom,
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

        let sohm_address = contract.config()?.sohm_address;
        stake_and_claim(&contract, 10_000, Some(receiver.address().to_string()))?;

        // We send some tokens to the contract, this should double the exchange rate
        chain.bank_send(
            contract.address()?.to_string(),
            coins(10_000, contract.config()?.ohm_denom),
        )?;
        stake_and_claim(&contract, 10_000, Some(receiver.address().to_string()))?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address,
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

        let sohm_address = contract.config()?.sohm_address;
        stake_and_claim(&contract, 10_000, Some(receiver.address().to_string()))?;

        // We send some tokens to the contract, this should double the exchange rate
        const SEND_TOKENS: u128 = 2_563;
        chain.bank_send(
            contract.address()?.to_string(),
            coins(SEND_TOKENS, contract.config()?.ohm_denom),
        )?;
        stake_and_claim(&contract, 5_000, Some(receiver.address().to_string()))?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address,
            10_000 + 5_000 * 10_000 / (10_000 + SEND_TOKENS),
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

        let sohm_address = contract.config()?.sohm_address;
        let ohm_denom = contract.config()?.ohm_denom;
        chain.bank_send(
            receiver.address().to_string(),
            coins(5_000, ohm_denom.clone()),
        )?;
        stake_and_claim(&contract, 10_000, None)?;
        stake_and_claim(
            &contract.call_as(&receiver),
            5_000,
            Some(receiver.address().to_string()),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            5000,
            receiver.address().to_string(),
        )?;
        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            10000,
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

        let sohm_address = contract.config()?.sohm_address;
        let ohm_denom = contract.config()?.ohm_denom;
        chain.bank_send(
            receiver.address().to_string(),
            coins(5_000, ohm_denom.clone()),
        )?;

        stake_and_claim(&contract, 10_000, None)?;

        stake_and_claim(
            &contract.call_as(&receiver),
            5_000,
            Some(receiver.address().to_string()),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            5_000,
            receiver.address().to_string(),
        )?;
        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            10_000,
            chain.sender().to_string(),
        )?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            0,
            receiver.address().to_string(),
        )?;

        unstake(&contract, 10_000, None)?;

        unstake(
            &contract.call_as(&receiver),
            5_000,
            Some(receiver.address().to_string()),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            receiver.address().to_string(),
        )?;
        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
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

        let sohm_address = contract.config()?.sohm_address;
        let ohm_denom = contract.config()?.ohm_denom;
        chain.bank_send(
            receiver.address().to_string(),
            coins(500_000, ohm_denom.clone()),
        )?;

        stake_and_claim(&contract, 500_000, None)?;
        stake_and_claim(
            &contract.call_as(&receiver),
            500_000,
            Some(receiver.address().to_string()),
        )?;

        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            1_000_000,
            contract.address()?.to_string(),
        )?;
        // We advance time to make sure we can rebase
        chain.wait_blocks(EPOCH_LENGTH)?;
        contract.rebase()?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            1_100_000,
            contract.address()?.to_string(),
        )?;
        println!(
            "rebase{:?}, now{:?}",
            contract.epoch_state()?,
            chain.block_info()?
        );

        unstake(&contract, 500_000, None)?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            contract.address()?.to_string(),
        )?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            550_000,
            contract.address()?.to_string(),
        )?;

        chain.wait_seconds(EPOCH_LENGTH)?;
        contract.rebase()?;

        unstake(
            &contract.call_as(&receiver),
            500_000,
            Some(receiver.address().to_string()),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            contract.address()?.to_string(),
        )?;

        assert_cw20_balance(
            chain.clone(),
            sohm_address.clone(),
            0,
            receiver.address().to_string(),
        )?;
        assert_cw20_balance(chain.clone(), sohm_address, 0, chain.sender().to_string())?;
        assert_balance(
            chain.clone(),
            ohm_denom.clone(),
            550000,
            chain.sender().to_string(),
        )?;
        assert_balance(chain, ohm_denom, 605000, receiver.address().to_string())?;

        Ok(())
    }
}
