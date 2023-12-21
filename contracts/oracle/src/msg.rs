use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal256, Uint128};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub first_epoch_time: u64,
    pub initial_balances: Vec<(String, Uint128)>,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    UpdatePair {
        token: String,
        denom1: String,
        denom2: String,
    },
    UpdatePool {
        total_share: Uint128,
        amount1: Uint128,
        amount2: Uint128,
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
    #[returns(PriceResponse)]
    Price {
        pair: String,
        token_in: String,
        token_out: String,
    },
}

#[cw_serde]
pub struct PriceResponse {
    pub current_price: Decimal256,
}
