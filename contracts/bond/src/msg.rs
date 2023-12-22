use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal256, Uint128};

use crate::state::{Config, Term};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub usd: String,
    pub principle: String,
    pub oracle: String,
    pub admin: Option<String>,
    pub staking: String,
    pub oracle_trust_period: u64, // We recommend something along the lines of 10 minutes = 600)
    pub term: Term,
    pub treasury: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    Redeem {
        recipient: String,
        stake: bool,
    },
    #[cfg_attr(feature = "interface", payable)]
    Deposit {
        max_price: Decimal256,
        depositor: String,
    },
    UpdateTerms {
        terms: Term,
    },
    UpdateConfig {
        usd: Option<String>,
        principle: Option<String>,
        admin: Option<String>,
        staking: Option<String>,
        oracle: Option<String>,
        oracle_trust_period: Option<u64>,
        treasury: Option<String>,
    },
    UpdateAdjustment {
        add: Option<bool>,
        rate: Option<Decimal256>,
        target: Option<Decimal256>,
        buffer: Option<u64>,
    },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(Uint128)]
    MaxPayout {},
    #[returns(Uint128)]
    PayoutFor { value: Uint128 },
    #[returns(Decimal256)]
    BondPrice {},
    #[returns(Decimal256)]
    AssetPrice {},
    #[returns(Decimal256)]
    DebtRatio {},
    #[returns(Decimal256)]
    StandardizedDebtRatio {},
    #[returns(Uint128)]
    CurrentDebt {},
    #[returns(Decimal256)]
    DebtDecay {},
    #[returns(Decimal256)]
    PercentVestedFor { recipient: String },
    #[returns(Decimal256)]
    PendingPayoutFor { recipient: String },
}
