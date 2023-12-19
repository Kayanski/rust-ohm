#[cfg(test)]
mod tests {
    use cosmwasm_std::Uint128;
    use cw_orch::{mock::Mock, prelude::*};
    use scw20::{
        interface::SCW20,
        msg::{ExecuteMsgFns, InstantiateMsg},
    };

    pub const ADMIN: &str = "ADMIN;=";

    #[test]
    pub fn first_integration() -> anyhow::Result<()> {
        let chain = Mock::new(&Addr::unchecked(ADMIN));

        let contract = SCW20::new("s-ohm", chain);

        contract.upload()?;
        contract.instantiate(
            &InstantiateMsg {
                base: cw20_base::msg::InstantiateMsg {
                    name: "OHM".to_string(),
                    symbol: "OHM".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: None,
                    marketing: None,
                },
            },
            None,
            None,
        )?;

        contract.rebase(Uint128::from(100u128))?;

        Ok(())
    }
}
