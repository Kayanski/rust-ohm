use cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance;
use cosmos_sdk_proto::traits::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
};
use injective_std::types::injective::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint};

use crate::error::{ContractError, ContractResult};
use crate::execute::{stake, unstake};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{query_config, query_exchange_rate, staking_denom};
use crate::state::{Config, Term, CONFIG, CURRENT_DEBT, LAST_DECAY, STAKING_TOKEN_DENOM, TERMS};
/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        principle: msg.principle,
        pair: deps.api.addr_validate(&msg.pair)?,
        admin: deps.api.addr_validate(&msg.admin)?,
        staking: deps.api.addr_validate(&msg.staking)?,
    };

    CONFIG.save(deps.storage, &config)?;
    TERMS.save(deps.storage, &msg.term)?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;
    CURRENT_DEBT.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new())
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
        ExecuteMsg::Deposit {
            max_price,
            depositor,
        } => deposit(deps, env, info, max_price, depositor),
        ExecuteMsg::Stake { to } => stake(deps, env, info, to),
        ExecuteMsg::Unstake { to } => unstake(deps, env, info, to),
        ExecuteMsg::UpdateTerms { terms } => update_terms(deps, info, terms),
        ExecuteMsg::UpdateConfig {
            principle,
            pair,
            admin,
            staking,
        } => update_config(deps, info, principle, pair, admin, staking),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::ExchangeRate {} => Ok(to_json_binary(&query_exchange_rate(deps, env)?)?),
    }
}

pub fn update_terms(deps: DepsMut, info: MessageInfo, terms: Term) -> ContractResult {
    TERMS.save(deps.storage, &terms)?;
    Ok(Response::new())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    principle: Option<String>,
    pair: Option<String>,
    admin: Option<String>,
    staking: Option<String>,
) -> ContractResult {
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(principle) = principle {
        config.principle = principle;
    }

    if let Some(pair) = pair {
        config.principle = pair;
    }

    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }

    if let Some(staking) = staking {
        config.staking = deps.api.addr_validate(&staking)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg(test)]
pub mod test {
    use anyhow::bail;
    use cosmwasm_std::coins;
    use cw_orch::{injective_test_tube::InjectiveTestTube, prelude::*};

    use cw_orch::injective_test_tube::injective_test_tube::Account;
    use staking::interface::Staking;
    use staking::msg::ExecuteMsgFns;
    use staking::msg::InstantiateMsg;
    use staking::msg::QueryMsgFns;
    use tests::tokenfactory::mint_denom;
    use tests::tokenfactory::tokenfactory_denom;
    use tests::tokenfactory::{assert_balance, create_denom};
    pub const MAIN_TOKEN: &str = "OHM";
    pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;
    pub const FUNDS_MULTIPLIER: u128 = 100_000;

    pub fn init() -> anyhow::Result<Staking<InjectiveTestTube>> {
        let chain = InjectiveTestTube::new(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"));

        // First we need to create the OHM denom
        create_denom(chain.clone(), MAIN_TOKEN.to_string())?;

        let contract = Staking::new("staking", chain.clone());
        contract.upload()?;

        contract.instantiate(
            &InstantiateMsg {
                ohm: format!("factory/{}/{MAIN_TOKEN}", chain.sender()),
                admin: None,
            },
            None,
            Some(&coins(AMOUNT_TO_CREATE_DENOM, "inj")),
        )?;

        Ok(contract)
    }

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        init()?;

        Ok(())
    }

    #[test]
    pub fn stake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        // We mint some MAIN_TOKEN
        mint_denom(chain.clone(), MAIN_TOKEN.to_string(), 100_000)?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        assert_balance(chain, sohm_denom, 10_000, receiver.address().to_string())?;

        Ok(())
    }

    #[test]
    pub fn unstake_works() -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let chain = contract.get_chain().clone();
        let sender = chain.sender();
        // We mint some MAIN_TOKEN
        mint_denom(chain.clone(), MAIN_TOKEN.to_string(), 100_000)?;

        contract.stake(
            sender.to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        let sohm_denom = contract.config()?.sohm;

        contract.unstake(sender.to_string(), &coins(10_000, sohm_denom.clone()))?;

        assert_balance(chain.clone(), sohm_denom, 0, sender.to_string())?;
        assert_balance(
            chain.clone(),
            tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            100_000,
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

        // We mint some MAIN_TOKEN
        mint_denom(chain.clone(), MAIN_TOKEN.to_string(), 100_000)?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
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
            coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        contract.stake(
            receiver.address().to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
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

        // We mint some MAIN_TOKEN
        mint_denom(chain.clone(), MAIN_TOKEN.to_string(), 100_000)?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
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
            coins(
                2_563,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        contract.stake(
            receiver.address().to_string(),
            &coins(
                10_000,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom,
            10_000 + 10_000 * 10_000 / (10_000 + 2_563),
            receiver.address().to_string(),
        )?;

        Ok(())
    }

    #[test_fuzz::test_fuzz]
    pub fn fuzz_stake_and_feed(
        first_stake: u128,
        feed: u128,
        second_stake: u128,
    ) -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        // We mint some MAIN_TOKEN
        mint_denom(
            chain.clone(),
            MAIN_TOKEN.to_string(),
            first_stake + second_stake + feed,
        )?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(
                first_stake,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            first_stake,
            receiver.address().to_string(),
        )?;

        // We send some tokens to the contract, this should double the exchange rate
        chain.bank_send(
            contract.address()?.to_string(),
            coins(
                feed,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        contract.stake(
            receiver.address().to_string(),
            &coins(
                second_stake,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom,
            first_stake + second_stake * first_stake / (first_stake + feed),
            receiver.address().to_string(),
        )?;

        Ok(())
    }

    #[test_fuzz::test_fuzz]
    fn fuzz_stake_and_feed_unstake(
        first_stake: u128,
        feed: u128,
        unstake: u128,
    ) -> anyhow::Result<()> {
        let contract: Staking<InjectiveTestTube> = init()?;
        let mut chain = contract.get_chain().clone();
        let receiver =
            chain.init_account(coins(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"))?;

        // We mint some MAIN_TOKEN
        mint_denom(chain.clone(), MAIN_TOKEN.to_string(), first_stake)?;

        let sohm_denom = contract.config()?.sohm;
        contract.stake(
            receiver.address().to_string(),
            &coins(
                first_stake + feed,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        assert_balance(
            chain.clone(),
            sohm_denom.clone(),
            first_stake,
            receiver.address().to_string(),
        )?;

        // We send some tokens to the contract, this should double the exchange rate
        chain.bank_send(
            contract.address()?.to_string(),
            coins(
                feed,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        )?;

        let unstake_response = contract.unstake(
            receiver.address().to_string(),
            &coins(
                unstake,
                tokenfactory_denom(chain.clone(), MAIN_TOKEN.to_string()),
            ),
        );

        if unstake > first_stake {
            if unstake_response.is_ok() {
                bail!("Unstake is higher than stake and we have an ok response on unstake")
            }
            assert_balance(
                chain.clone(),
                sohm_denom,
                first_stake,
                receiver.address().to_string(),
            )?;
        } else {
            if unstake_response.is_err() {
                bail!("when unstake is lower than stake, unstaking should always be allowed ")
            }
            assert_balance(
                chain.clone(),
                sohm_denom,
                first_stake - unstake * (first_stake + feed) / first_stake,
                receiver.address().to_string(),
            )?;
        }

        Ok(())
    }
}
