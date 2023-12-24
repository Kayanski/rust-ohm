use cw_orch::daemon::{networks::injective::INJECTIVE_NETWORK, ChainInfo, ChainKind};

pub mod deploy;
pub mod exchange;
pub mod tokenfactory;
pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;

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

#[cfg(test)]
mod tests {}
