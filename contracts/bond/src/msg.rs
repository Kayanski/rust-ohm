use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal256, Uint128};

use crate::state::{Adjustment, Bond, Terms};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub principle: String,
    pub admin: Option<String>,
    pub staking: String,
    pub terms: Terms,
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
        terms: Terms,
    },
    UpdateConfig {
        principle: Option<String>,
        admin: Option<String>,
        staking: Option<String>,
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
    #[returns(ConfigResponse)]
    Config {},
    #[returns(Terms)]
    Terms {},
    #[returns(Adjustment)]
    Adjustment {},
    #[returns(Uint128)]
    MaxPayout {},
    #[returns(Uint128)]
    PayoutFor { value: Uint128 },
    #[returns(Decimal256)]
    BondPrice {},
    #[returns(Decimal256)]
    DebtRatio {},
    #[returns(Uint128)]
    CurrentDebt {},
    #[returns(Decimal256)]
    DebtDecay {},
    #[returns(Decimal256)]
    PercentVestedFor { recipient: String },
    #[returns(Decimal256)]
    PendingPayoutFor { recipient: String },
    #[returns(Bond)]
    BondInfo { recipient: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub principle: String,
    pub admin: String,
    pub staking: String,
    pub treasury: String,
}
