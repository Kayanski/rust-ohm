use cosmwasm_std::coins;
use cw_orch::{
    daemon::{networks::injective::INJECTIVE_NETWORK, ChainInfo, ChainKind},
    injective_test_tube::InjectiveTestTube,
};

pub mod deploy;
pub mod exchange;
#[cfg(test)]
pub mod integration;
pub mod tokenfactory;
pub const AMOUNT_TO_CREATE_DENOM_TEST: u128 = 10_000_000_000_000_000_000u128;
pub const FUNDS_MULTIPLIER: u128 = 100_000;

pub const LOCAL_INJECTIVE: ChainInfo = ChainInfo {
    kind: ChainKind::Local,
    chain_id: "injective-1",
    gas_denom: "inj",
    gas_price: 500_000_000.0,
    grpc_urls: &["http://localhost:9900"],
    network_info: INJECTIVE_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

pub fn test_tube() -> InjectiveTestTube {
    InjectiveTestTube::new(coins(AMOUNT_TO_CREATE_DENOM_TEST * FUNDS_MULTIPLIER, "inj"))
}

#[cfg(test)]
mod tests {}
