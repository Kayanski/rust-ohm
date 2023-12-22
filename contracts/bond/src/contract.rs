#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
};

use crate::error::{ContractError, ContractResult, QueryResult};
use crate::execute::{current_debt, debt_decay, deposit, redeem};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{
    asset_price, debt_ratio, max_payout, payout_for, pending_payout_for, percent_vested_for,
    query_config, standardized_debt_ratio,
};
use crate::state::{
    query_bond_price, Config, Term, ADJUSTMENT, CONFIG, LAST_DECAY, TERMS, TOTAL_DEBT,
};
/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        usd: msg.usd,
        principle: msg.principle,
        oracle: deps.api.addr_validate(&msg.oracle)?,
        admin: msg
            .admin
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .unwrap_or(info.sender),
        staking: deps.api.addr_validate(&msg.staking)?,
        oracle_trust_period: msg.oracle_trust_period,
        treasury: deps.api.addr_validate(&msg.treasury)?,
    };

    CONFIG.save(deps.storage, &config)?;
    TERMS.save(deps.storage, &msg.term)?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;
    TOTAL_DEBT.save(deps.storage, &Uint128::zero())?;

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
        ExecuteMsg::Redeem { recipient, stake } => redeem(deps, env, info, recipient, stake),
        ExecuteMsg::UpdateTerms { terms } => update_terms(deps, info, terms),
        ExecuteMsg::UpdateConfig {
            principle,
            pair,
            admin,
            staking,
        } => update_config(deps, info, principle, pair, admin, staking),
        ExecuteMsg::UpdateAdjustment {
            add,
            rate,
            target,
            buffer,
        } => update_adjustment(deps, info, add, rate, target, buffer),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::MaxPayout {} => Ok(to_json_binary(&max_payout(deps)?)?),
        QueryMsg::PayoutFor { value } => Ok(to_json_binary(&payout_for(deps, env, value)?)?),
        QueryMsg::BondPrice {} => Ok(to_json_binary(&query_bond_price(deps, env)?)?),
        QueryMsg::AssetPrice {} => Ok(to_json_binary(&asset_price(deps, env)?)?),
        QueryMsg::DebtRatio {} => Ok(to_json_binary(&debt_ratio(deps, env)?)?),
        QueryMsg::StandardizedDebtRatio {} => {
            Ok(to_json_binary(&standardized_debt_ratio(deps, env)?)?)
        }
        QueryMsg::CurrentDebt {} => Ok(to_json_binary(&current_debt(deps, env)?)?),
        QueryMsg::DebtDecay {} => Ok(to_json_binary(&debt_decay(deps, env)?)?),
        QueryMsg::PercentVestedFor { recipient } => Ok(to_json_binary(&percent_vested_for(
            deps,
            env,
            &deps.api.addr_validate(&recipient)?,
        )?)?),
        QueryMsg::PendingPayoutFor { recipient } => {
            Ok(to_json_binary(&pending_payout_for(deps, env, recipient)?)?)
        }
    }
}

pub fn update_terms(deps: DepsMut, info: MessageInfo, terms: Term) -> ContractResult {
    if info.sender != CONFIG.load(deps.storage)?.admin {
        return Err(ContractError::Unauthorized {});
    }
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
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

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

pub fn update_adjustment(
    deps: DepsMut,
    info: MessageInfo,
    add: Option<bool>,
    rate: Option<Decimal256>,
    target: Option<Decimal256>,
    buffer: Option<u64>,
) -> ContractResult {
    if info.sender != CONFIG.load(deps.storage)?.admin {
        return Err(ContractError::Unauthorized {});
    }
    let mut adjustment = ADJUSTMENT.load(deps.storage)?;

    if let Some(add) = add {
        adjustment.add = add;
    }

    if let Some(rate) = rate {
        adjustment.rate = rate;
    }

    if let Some(target) = target {
        adjustment.target = target;
    }

    if let Some(buffer) = buffer {
        adjustment.buffer = buffer;
    }

    ADJUSTMENT.save(deps.storage, &adjustment)?;

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
