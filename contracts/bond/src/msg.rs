use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal256;

use crate::state::{Config, Term};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub usd: String,
    pub principle: String,
    pub oracle: String,
    pub admin: String,
    pub staking: String,
    pub oracle_trust_period: u64, // We recommend something along the lines of 10 minutes = 600)
    pub term: Term,
    pub treasury: String,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    #[cfg_attr(feature = "interface", payable)]
    Stake {
        to: String,
    },
    #[cfg_attr(feature = "interface", payable)]
    Unstake {
        to: String,
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
        principle: Option<String>,
        pair: Option<String>,
        admin: Option<String>,
        staking: Option<String>,
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
    #[returns(Decimal256)]
    ExchangeRate {},
}