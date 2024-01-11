use std::rc::Rc;
use std::str::FromStr;

use super::unstake;
use crate::deploy::upload::BondConfig;
use crate::integration::test_constants::bond_terms_1::VESTING_TERM;
use crate::integration::test_constants::*;
use crate::tokenfactory::{assert_cw20_balance, get_cw20_balance};
use crate::{
    deploy::upload::{Shogun, ShogunDeployment},
    test_tube,
    tokenfactory::assert_balance,
    AMOUNT_TO_CREATE_DENOM_TEST,
};
use bond::interface::Bond;
use bond::msg::ExecuteMsgFns as _;
use bond::msg::QueryMsgFns as _;
use bond::state::Adjustment;
use bond::state::Terms;
use cosmwasm_std::{coins, Decimal256, Timestamp, Uint128};
use cw_orch::injective_test_tube::injective_test_tube::{Account, SigningAccount};
use cw_orch::injective_test_tube::InjectiveTestTube;
use cw_orch::{
    contract::interface_traits::ContractInstance, deploy::Deploy, environment::TxHandler,
    prelude::*,
};
use cw_plus_interface::cw1_whitelist::Cw1Whitelist;
use staking_contract::msg::BondContractsElem;
use staking_contract::msg::ExecuteMsgFns as _;
use staking_contract::msg::QueryMsgFns as _;
use staking_contract::state::StakingPoints;

pub const WARMUP_LENGTH: u64 = 50;

pub fn init() -> anyhow::Result<Shogun<InjectiveTestTube>> {
    let chain = test_tube();

    let cw1 = Cw1Whitelist::new("cw1-whitelist", chain.clone());
    cw1.upload()?;

    let shogun = Shogun::deploy_on(
        chain.clone(),
        ShogunDeployment {
            epoch_length: EPOCH_LENGTH,
            first_epoch_time: chain.block_info()?.time.seconds() + EPOCH_LENGTH,
            epoch_apr: Decimal256::from_str(EPOCH_APR)?,
            initial_balances: vec![(chain.sender().to_string(), INITIAL_SHOGUN_BALANCE)],
            amount_to_create_denom: AMOUNT_TO_CREATE_DENOM_TEST,
            fee_token: "inj".to_string(),
            staking_symbol: "sSHGN".to_string(),
            staking_name: "sSHOGUN".to_string(),
            warmup_length: WARMUP_LENGTH,
            cw1_code_id: cw1.code_id()?,
        },
    )?;

    Ok(shogun)
}

pub fn init_bond() -> anyhow::Result<(
    Shogun<InjectiveTestTube>,
    Bond<InjectiveTestTube>,
    Rc<SigningAccount>,
)> {
    let mut shogun = init()?;
    let mut chain = shogun.staking.get_chain().clone();

    let _ = pretty_env_logger::try_init();

    let treasury = chain.init_account(vec![])?;

    let bond_denom = bond_terms_1::BOND_TOKEN.to_string();
    shogun.add_bond(
        chain.clone(),
        BondConfig {
            bond_token_denom: bond_denom.clone(),
            treasury: treasury.address().to_string(),
            terms: Terms {
                control_variable: Decimal256::from_str(bond_terms_1::CONTROL_VARIABLE)?,
                max_debt: bond_terms_1::MAX_DEBT.into(),
                max_payout: Decimal256::from_str(bond_terms_1::MAX_PAYOUT)?,
                minimum_price: Decimal256::from_str(bond_terms_1::MINIMUM_PRICE)?,
                vesting_term: bond_terms_1::VESTING_TERM,
            },
        },
    )?;

    let bond_contract = shogun.bonds.get(&bond_denom).unwrap();

    Ok((shogun.clone(), bond_contract.clone(), treasury))
}

#[test]
fn deploy_check() -> anyhow::Result<()> {
    let shogun = init()?;
    let chain = shogun.staking.get_chain();

    shogun.staking.address()?;
    shogun.staking_token.address()?;
    shogun.bond_code_id()?;

    let config = shogun.staking.config()?;

    // We verify all variables are set correctly
    assert_eq!(
        config,
        staking_contract::msg::ConfigResponse {
            admin: chain.sender().to_string(),
            epoch_apr: Decimal256::from_str(EPOCH_APR)?,
            next_epoch_apr: None,
            epoch_length: EPOCH_LENGTH,
            ohm_denom: shogun.staking.config()?.ohm_denom,
            sohm_address: shogun.staking.config()?.sohm_address,
            warmup_address: shogun.staking.config()?.warmup_address,
            warmup_length: WARMUP_LENGTH
        }
    );

    let epoch_state = shogun.staking.epoch_state()?;

    assert_eq!(epoch_state.epoch_number, 0);
    assert_eq!(epoch_state.epoch_start, epoch_state.epoch_end);

    assert_balance(
        chain.clone(),
        config.ohm_denom,
        INITIAL_SHOGUN_BALANCE,
        chain.sender().to_string(),
    )?;
    assert_cw20_balance(
        chain.clone(),
        config.sohm_address,
        0,
        chain.sender().to_string(),
    )?;

    Ok(())
}

#[test]
fn bond_check() -> anyhow::Result<()> {
    let (shogun, bond_contract, treasury) = init_bond()?;
    let chain = shogun.staking.get_chain().clone();

    let bond_denom = bond_terms_1::BOND_TOKEN.to_string();

    let bond_config = bond_contract.config()?;

    assert_eq!(
        bond_config,
        bond::msg::ConfigResponse {
            admin: chain.sender().to_string(),
            principle: bond_denom.clone(),
            staking: shogun.staking.address()?.to_string(),
            treasury: treasury.address().to_string(),
        }
    );

    let bond_terms = bond_contract.terms()?;

    assert_eq!(
        bond_terms,
        Terms {
            control_variable: Decimal256::from_str(bond_terms_1::CONTROL_VARIABLE)?,
            max_debt: bond_terms_1::MAX_DEBT.into(),
            max_payout: Decimal256::from_str(bond_terms_1::MAX_PAYOUT)?,
            minimum_price: Decimal256::from_str(bond_terms_1::MINIMUM_PRICE)?,
            vesting_term: bond_terms_1::VESTING_TERM,
        }
    );

    let bonds = shogun.staking.bonds()?;

    assert_eq!(
        bonds.bonds,
        vec![BondContractsElem {
            bond_token: bond_denom.clone(),
            bond_address: bond_contract.address()?.to_string(),
        }]
    );

    Ok(())
}

#[test]
fn modify_staking_config() -> anyhow::Result<()> {
    let shogun = init()?;
    let mut chain = shogun.staking.get_chain().clone();
    let new_admin = chain.init_account(vec![])?;

    let new_apr = Decimal256::from_str("1.3493859798")?;
    let new_epoch_length = 842387;

    shogun.staking.update_config(
        None,
        Some(new_admin.address().to_string()),
        Some(new_apr),
        Some(new_epoch_length),
        None,
    )?;

    assert_eq!(
        shogun.staking.config()?,
        staking_contract::msg::ConfigResponse {
            admin: new_admin.address().to_string(),
            epoch_apr: Decimal256::from_str(EPOCH_APR)?,
            next_epoch_apr: Some(new_apr),
            epoch_length: new_epoch_length,
            ohm_denom: shogun.staking.config()?.ohm_denom,
            sohm_address: shogun.staking.config()?.sohm_address,
            warmup_address: shogun.staking.config()?.warmup_address,
            warmup_length: WARMUP_LENGTH
        }
    );

    Ok(())
}

#[test]
fn modify_bond_config() -> anyhow::Result<()> {
    let (shogun, bond_contract, _treasury) = init_bond()?;
    let mut chain = shogun.staking.get_chain().clone();
    let new_admin = chain.init_account(vec![])?;
    let new_treasury = chain.init_account(vec![])?;
    let new_staking = chain.init_account(vec![])?;

    let new_principle = "new_principle".to_string();

    bond_contract.update_config(
        Some(new_admin.address().to_string()),
        Some(new_principle.clone()),
        Some(new_staking.address().to_string()),
        Some(new_treasury.address().to_string()),
    )?;

    assert_eq!(
        bond_contract.config()?,
        bond::msg::ConfigResponse {
            admin: new_admin.address().to_string(),
            principle: new_principle,
            staking: new_staking.address().to_string(),
            treasury: new_treasury.address().to_string(),
        }
    );
    Ok(())
}

#[test]
fn modify_bond_adjust() -> anyhow::Result<()> {
    let (shogun, bond_contract, _treasury) = init_bond()?;
    let chain = shogun.staking.get_chain().clone();

    let new_add = false;
    let new_target = Decimal256::from_str("4736476")?;
    let new_rate = Decimal256::from_str("473624862786476")?;
    let new_buffer = 427u64;

    bond_contract.update_adjustment(
        Some(new_add),
        Some(new_buffer),
        Some(new_rate),
        Some(new_target),
    )?;

    assert_eq!(
        bond_contract.adjustment()?,
        Adjustment {
            add: new_add,
            buffer: new_buffer,
            rate: new_rate,
            target: new_target,
            last_time: Timestamp::from_seconds(chain.block_info()?.time.seconds() - 2)
        }
    );
    Ok(())
}

#[test]
fn modify_bond_terms() -> anyhow::Result<()> {
    let (_shogun, bond_contract, _treasury) = init_bond()?;

    let new_control_variable = Decimal256::from_str("325437867")?;
    let new_max_debt = 1398357897u128;
    let new_max_payout = Decimal256::from_str("7238247")?;
    let new_minimum_price = Decimal256::from_str("1387364876")?;
    let new_vesting_term = 24982987u64;

    bond_contract.update_terms(Terms {
        control_variable: new_control_variable,
        max_debt: new_max_debt.into(),
        max_payout: new_max_payout,
        minimum_price: new_minimum_price,
        vesting_term: new_vesting_term,
    })?;

    assert_eq!(
        bond_contract.terms()?,
        Terms {
            control_variable: new_control_variable,
            max_debt: new_max_debt.into(),
            max_payout: new_max_payout,
            minimum_price: new_minimum_price,
            vesting_term: new_vesting_term
        }
    );

    Ok(())
}

#[test]
fn full_operations() -> anyhow::Result<()> {
    let (shogun, bond_contract, _treasury) = init_bond()?;

    let mut chain = shogun.staking.get_chain().clone();
    let recipient = chain.init_account(vec![])?;
    // Stake users
    shogun.staking.stake(
        chain.sender().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;
    shogun.staking.stake(
        recipient.address().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;

    // use a bond. Assert that the price goes up when more bonds are used
    let bond_price_before = bond_contract.bond_price()?;
    bond_contract.deposit(
        recipient.address().to_string(),
        Decimal256::from_str("2.2")?,
        &coins(10_000, bond_terms_1::BOND_TOKEN),
    )?;
    let bond_price_after = bond_contract.bond_price()?;
    println!("{} - {}", bond_price_before, bond_price_after);
    assert!(bond_price_before < bond_price_after);

    // Check that the bond goes down after some time
    chain.wait_blocks(VESTING_TERM / 10)?;
    let bond_price_final = bond_contract.bond_price()?;
    assert!(bond_price_final < bond_price_after);

    // Rebase and check that exchange rate goes up
    let exchange_rate_before = shogun.staking.exchange_rate()?;
    shogun.staking.rebase()?;
    let exchange_rate_after = shogun.staking.exchange_rate()?;
    assert!(exchange_rate_before < exchange_rate_after);

    Ok(())
}

#[test]
fn no_stake_rebase() -> anyhow::Result<()> {
    let (shogun, _bond_contract, _treasury) = init_bond()?;

    let chain = shogun.staking.get_chain().clone();

    chain.wait_seconds(EPOCH_LENGTH)?;
    shogun.staking.rebase()?;

    // We verify the last epoch updated is now
    let epoch_state = shogun.staking.epoch_state()?;
    assert_eq!(
        epoch_state.epoch_end.seconds(),
        chain.block_info()?.time.seconds() + EPOCH_LENGTH - 9 // To account for passed blocks counted by injective test tube
    );

    Ok(())
}

#[test]
fn hop_before_rebase_test() -> anyhow::Result<()> {
    let (shogun, _bond_contract, _treasury) = init_bond()?;

    let mut chain = shogun.staking.get_chain().clone();

    chain.wait_seconds(EPOCH_LENGTH)?;
    shogun.staking.rebase()?;

    // We stake before for the recipient
    let recipient = chain.init_account(vec![])?;
    shogun.staking.stake(
        recipient.address().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;

    // We wait some time just before the epoch end
    chain.wait_seconds(EPOCH_LENGTH - 10)?;

    // We stake after for the sender
    shogun.staking.stake(
        chain.sender().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;

    // We rebase just there
    shogun.staking.rebase()?;

    // The balance should not be equal for the 2 participants
    let config = shogun.staking.config()?;

    assert_ne!(
        get_cw20_balance(
            chain.clone(),
            config.sohm_address.clone(),
            chain.sender().to_string(),
        )?,
        get_cw20_balance(
            chain.clone(),
            config.sohm_address,
            recipient.address().to_string(),
        )?
    );

    Ok(())
}

#[test]
fn points_increase_and_stop() -> anyhow::Result<()> {
    let (shogun, _bond_contract, _treasury) = init_bond()?;
    let chain = shogun.staking.get_chain().clone();

    assert_eq!(
        shogun
            .staking
            .raw_staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: chain.block_info()?.time
        }
    );
    assert_eq!(
        shogun.staking.staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: chain.block_info()?.time
        }
    );

    shogun.staking.stake(
        chain.sender().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;

    assert_eq!(
        shogun
            .staking
            .raw_staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: chain.block_info()?.time
        }
    );
    assert_eq!(
        shogun.staking.staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::one(),
            last_points_updated: chain.block_info()?.time
        }
    );

    unstake(&shogun, 1_000, None)?;

    assert_eq!(
        shogun
            .staking
            .raw_staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::zero(),
            last_points_updated: chain.block_info()?.time
        }
    );
    assert_eq!(
        shogun.staking.staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::one(),
            last_points_updated: chain.block_info()?.time
        }
    );
    shogun.staking.stake(
        chain.sender().to_string(),
        &coins(10_000, shogun.staking.config()?.ohm_denom),
    )?;
    assert_eq!(
        shogun
            .staking
            .raw_staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::one(),
            last_points_updated: chain.block_info()?.time
        }
    );
    assert_eq!(
        shogun.staking.staking_points(chain.sender().to_string(),)?,
        StakingPoints {
            total_points: Uint128::one() + Uint128::one(),
            last_points_updated: chain.block_info()?.time
        }
    );

    Ok(())
}
