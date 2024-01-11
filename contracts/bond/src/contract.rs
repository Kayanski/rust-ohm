#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
};

use crate::error::{ContractError, ContractResult, QueryResult};
use crate::execute::{current_debt, debt_decay, deposit, redeem};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{
    bond_info, debt_ratio, max_payout, payout_for, pending_payout_for, percent_vested_for,
    query_adjustment, query_config, query_terms,
};
use crate::state::{
    query_bond_price, Adjustment, Config, Terms, ADJUSTMENT, CONFIG, LAST_DECAY, TERMS, TOTAL_DEBT,
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
        principle: msg.principle,
        admin: msg
            .admin
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .unwrap_or(info.sender),
        staking: deps.api.addr_validate(&msg.staking)?,
        treasury: deps.api.addr_validate(&msg.treasury)?,
    };

    CONFIG.save(deps.storage, &config)?;
    TERMS.save(deps.storage, &msg.terms)?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;
    TOTAL_DEBT.save(deps.storage, &Uint128::zero())?;
    ADJUSTMENT.save(
        deps.storage,
        &Adjustment {
            add: true,
            rate: Decimal256::zero(),
            target: Decimal256::zero(),
            buffer: 0,
            last_time: env.block.time,
        },
    )?;

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
            admin,
            staking,
            treasury,
        } => update_config(deps, info, principle, admin, staking, treasury),
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
        QueryMsg::Terms {} => Ok(to_json_binary(&query_terms(deps)?)?),
        QueryMsg::Adjustment {} => Ok(to_json_binary(&query_adjustment(deps)?)?),
        QueryMsg::MaxPayout {} => Ok(to_json_binary(&max_payout(deps)?)?),
        QueryMsg::PayoutFor { value } => Ok(to_json_binary(&payout_for(deps, env, value)?)?),
        QueryMsg::BondPrice {} => Ok(to_json_binary(&query_bond_price(deps, env)?)?),
        QueryMsg::DebtRatio {} => Ok(to_json_binary(&debt_ratio(deps, env)?)?),
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
        QueryMsg::BondInfo { recipient } => Ok(to_json_binary(&bond_info(deps, recipient)?)?),
    }
}

pub fn update_terms(deps: DepsMut, info: MessageInfo, terms: Terms) -> ContractResult {
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
    admin: Option<String>,
    staking: Option<String>,
    treasury: Option<String>,
) -> ContractResult {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(principle) = principle {
        config.principle = principle;
    }
    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }
    if let Some(staking) = staking {
        config.staking = deps.api.addr_validate(&staking)?;
    }
    if let Some(treasury) = treasury {
        config.treasury = deps.api.addr_validate(&treasury)?;
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
    use std::rc::Rc;
    use std::str::FromStr;

    use cosmwasm_std::coin;
    use cosmwasm_std::coins;
    use cosmwasm_std::Decimal256;
    use cosmwasm_std::Uint128;
    use cw_orch::injective_test_tube::injective_test_tube::SigningAccount;
    use cw_orch::{injective_test_tube::InjectiveTestTube, prelude::*};

    use bond::interface::Bond;
    use bond::msg::ExecuteMsgFns as _;
    use bond::msg::QueryMsgFns;
    use bond::state::Terms;
    use cw_orch::injective_test_tube::injective_test_tube::Account;
    use cw_plus_interface::cw1_whitelist::Cw1Whitelist;
    use staking_contract::msg::QueryMsgFns as _;
    use tests::deploy::upload::BondConfig;
    use tests::deploy::upload::Shogun;
    use tests::deploy::upload::ShogunDeployment;
    use tests::tokenfactory::assert_balance;

    pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;
    pub const FUNDS_MULTIPLIER: u128 = 100_000;
    pub const BOND_TOKEN: &str = "ubond";
    pub const USD_TOKEN: &str = "uusd";

    pub fn init() -> anyhow::Result<(
        Shogun<InjectiveTestTube>,
        Bond<InjectiveTestTube>,
        Rc<SigningAccount>,
    )> {
        let mut chain = InjectiveTestTube::new(vec![
            coin(AMOUNT_TO_CREATE_DENOM * FUNDS_MULTIPLIER, "inj"),
            coin(10_000_000, BOND_TOKEN),
        ]);

        let block_info = chain.block_info()?;

        let treasury = chain.init_account(vec![])?;

        let cw1 = Cw1Whitelist::new("cw1-whitelist", chain.clone());
        cw1.upload()?;

        let mut shogun = Shogun::deploy_on(
            chain.clone(),
            ShogunDeployment {
                epoch_length: 100,
                first_epoch_time: block_info.time.seconds() + 1,
                epoch_apr: Decimal256::from_str("0.1")?,
                initial_balances: vec![(chain.sender().to_string(), 1_000_000u128)],
                amount_to_create_denom: AMOUNT_TO_CREATE_DENOM,
                fee_token: "inj".to_string(),
                staking_symbol: "sSHGN".to_string(),
                staking_name: "sSHOGUN".to_string(),
                warmup_length: 50,
                cw1_code_id: cw1.code_id()?,
            },
        )?;

        shogun.add_bond(
            chain,
            BondConfig {
                bond_token_denom: BOND_TOKEN.to_string(),
                treasury: treasury.address(),
                terms: Terms {
                    control_variable: Decimal256::from_str("1000")?,
                    minimum_price: Decimal256::from_str("2")?,
                    max_payout: Decimal256::from_str("0.2")?,
                    max_debt: 500_000u128.into(),
                    vesting_term: 3600u64, // 1h
                },
            },
        )?;

        Ok((
            shogun.clone(),
            shogun.bonds.get(BOND_TOKEN).unwrap().clone(),
            treasury,
        ))
    }

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        init()?;

        Ok(())
    }

    #[test]
    pub fn bond_works() -> anyhow::Result<()> {
        let (_, bond, treasury) = init()?;
        let chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("2")?;
        assert_eq!(bond.bond_price()?, Decimal256::from_str("2")?);

        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        assert_balance(
            chain.clone(),
            BOND_TOKEN.to_string(),
            10_000,
            treasury.address().to_string(),
        )?;

        // assert bond exists and has the right terms
        let term = bond.bond_info(chain.sender().to_string())?;

        assert_eq!(
            term,
            bond::state::Bond {
                payout: 5_000u128.into(),
                vesting_time_left: 3600,
                last_time: chain.block_info()?.time
            }
        );

        assert!(bond.current_debt()? > Uint128::zero());

        Ok(())
    }

    #[test]
    pub fn not_lower_than_max_price() -> anyhow::Result<()> {
        let (_, bond, _treasury) = init()?;
        let chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("1.9")?;
        let err = bond
            .deposit(
                chain.sender().to_string(),
                max_price,
                &coins(10_000, BOND_TOKEN),
            )
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("Slippage limit: more than max price"));

        Ok(())
    }

    #[test]
    pub fn not_higher_than_max_capacity() -> anyhow::Result<()> {
        let (_, bond, _treasury) = init()?;
        let chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("2")?;
        let err = bond
            .deposit(
                chain.sender().to_string(),
                max_price,
                &coins(400_002, BOND_TOKEN),
            )
            .unwrap_err();
        assert!(err.to_string().contains("Bond too large"));

        Ok(())
    }

    #[test]
    pub fn not_too_much_debt() -> anyhow::Result<()> {
        let (_, bond, _treasury) = init()?;

        bond.update_terms(Terms {
            control_variable: Decimal256::from_str("1")?,
            minimum_price: Decimal256::from_str("2")?,
            max_payout: Decimal256::from_str("2")?,
            max_debt: 500_000u128.into(),
            vesting_term: 3600u64,
        })?;

        let chain = bond.get_chain().clone();

        // We deposit more than a payout of 500_000 to trigger an error (price=0.5)
        // We can't deposit more than 400_000 at a time though
        let max_price = Decimal256::from_str("2")?;
        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(1_000_002, BOND_TOKEN),
        )?;
        let err = bond
            .deposit(chain.sender().to_string(), max_price, &coins(1, BOND_TOKEN))
            .unwrap_err();

        assert!(err.to_string().contains("Max capacity reached"));

        Ok(())
    }

    #[test]
    pub fn bond_adds() -> anyhow::Result<()> {
        let (_, bond, treasury) = init()?;
        let chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("5")?;

        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        // assert bond exists and has the right terms
        let term = bond.bond_info(chain.sender().to_string())?;

        assert_balance(
            chain.clone(),
            BOND_TOKEN.to_string(),
            20_000,
            treasury.address().to_string(),
        )?;

        assert_eq!(
            term,
            bond::state::Bond {
                payout: 7010u128.into(),
                vesting_time_left: 3600,
                last_time: chain.block_info()?.time
            }
        );

        Ok(())
    }

    #[test]
    pub fn bond_adds_with_price_increase() -> anyhow::Result<()> {
        let (_shogun, bond, treasury) = init()?;
        let chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("2.5")?;

        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        chain.wait_blocks(180)?;

        assert_eq!(
            bond.percent_vested_for(chain.sender().to_string())?,
            Decimal256::from_str("0.5")?
        );

        bond.deposit(
            chain.sender().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        // assert bond exists and has the right terms
        let term = bond.bond_info(chain.sender().to_string())?;

        assert_balance(
            chain.clone(),
            BOND_TOKEN.to_string(),
            20_000,
            treasury.address().to_string(),
        )?;

        assert_eq!(term.payout.u128(), 9_021);

        Ok(())
    }

    #[test]
    pub fn unstake_works() -> anyhow::Result<()> {
        let (shogun, bond, _treasury) = init()?;
        let mut chain = bond.get_chain().clone();

        let max_price = Decimal256::from_str("2")?;
        let receiver = chain.init_account(vec![])?;

        bond.deposit(
            receiver.address().to_string(),
            max_price,
            &coins(10_000, BOND_TOKEN),
        )?;

        chain.wait_blocks(180)?;

        bond.redeem(receiver.address().to_string(), false)?;

        assert_balance(
            chain.clone(),
            shogun.staking.config()?.ohm_denom,
            2_501,
            receiver.address().to_string(),
        )?;

        bond.redeem(receiver.address().to_string(), false)
            .unwrap_err();

        chain.wait_blocks(180)?;

        bond.redeem(receiver.address().to_string(), false)?;
        assert_balance(
            chain.clone(),
            shogun.staking.config()?.ohm_denom,
            5_000,
            receiver.address().to_string(),
        )?;

        Ok(())
    }
}
