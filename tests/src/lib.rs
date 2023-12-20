pub mod tokenfactory;

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Uint128};
    use cw_orch::{injective_test_tube::InjectiveTestTube, mock::Mock, prelude::*};
    use scw20::{
        interface::SCW20,
        msg::{ExecuteMsgFns, InstantiateMsg},
    };

    #[test]
    pub fn first_integration() -> anyhow::Result<()> {
        let chain = InjectiveTestTube::new(coins(100_000_000_000_000, "inj"));

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

        // contract.rebase(Uint128::from(100u128))?;

        Ok(())
    }
}
