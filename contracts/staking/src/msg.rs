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
    #[cfg_attr(feature = "interface", payable)]
    Stake {
        to: String,
    },
    #[cfg_attr(feature = "interface", payable)]
    Unstake {
        to: String,
    },
    Rebase {},
    Mint {
        to: String,
        amount: Uint128,
    },
    UpdateConfig {
        admin: Option<String>,
        epoch_length: Option<u64>,
        epoch_apr: Option<Decimal256>,
        add_minter: Option<Vec<String>>,
        remove_minter: Option<Vec<String>>,
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
    #[returns(Decimal256)]
    ExchangeRate {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub admin: String,
    pub ohm: String,
    pub sohm: String,
}
