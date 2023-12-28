use std::{collections::HashMap, path::PathBuf};

use bond::interface::Bond;
use bond::state::Terms;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coins, Decimal256};
use cw_orch::{
    contract::interface_traits::{ContractInstance, CwOrchInstantiate, CwOrchUpload},
    deploy::Deploy,
    environment::CwEnv,
    prelude::CwOrchError,
};
use oracle::msg::QueryMsgFns as _;
use oracle::{interface::Oracle, msg::ExecuteMsgFns as _};
use staking::interface::Staking;
use staking::msg::BondContractsElem;
use staking::msg::ExecuteMsgFns as _;
use staking::msg::QueryMsgFns as _;

pub const BOND_CODE_ID: &str = "bond-code-id";

#[derive(Clone)]
pub struct Shogun<Chain: CwEnv> {
    pub staking: Staking<Chain>,
    pub bonds: HashMap<String, Bond<Chain>>,
    pub oracle: Oracle<Chain>,
}
impl<Chain: CwEnv> Deploy<Chain> for Shogun<Chain> {
    type Error = CwOrchError;

    type DeployData = ShogunDeployment;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let shogun = Self::new(chain);

        shogun.staking.upload()?;
        shogun.bonds.get(BOND_CODE_ID).unwrap().upload()?;
        shogun.oracle.upload()?;

        Ok(shogun)
    }

    fn deployed_state_file_path() -> Option<String> {
        let crate_path = env!("CARGO_MANIFEST_DIR");

        Some(
            PathBuf::from(crate_path)
                // State file of your deployment
                .join("cw-orch-state.json")
                .display()
                .to_string(),
        )
    }

    fn get_contracts_mut(
        &mut self,
    ) -> Vec<Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>>> {
        let staking_box: Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>> =
            Box::new(&mut self.staking);
        let oracle_box: Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>> =
            Box::new(&mut self.oracle);
        self.bonds
            .iter_mut()
            .map(|(_, d)| {
                let boxed: Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>> = Box::new(d);
                boxed
            })
            .chain([staking_box, oracle_box])
            .collect()
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let mut shogun = Self::new(chain.clone());

        // We get all the bonds contracts and their names
        let bonds = shogun.staking.bonds()?;

        // We create bond contracts
        shogun.bonds = bonds
            .bonds
            .into_iter()
            .map(|bond_config| {
                let bond_contract = Bond::new(
                    Self::bond_contract_name(bond_config.bond_token.clone()),
                    chain.clone(),
                );
                (bond_config.bond_token, bond_contract)
            })
            .collect();

        Ok(shogun)
    }

    fn deploy_on(chain: Chain, data: ShogunDeployment) -> Result<Self, Self::Error> {
        let deployment = Self::store_on(chain.clone())?;

        deployment.instantiate(data)?;

        Ok(deployment)
    }
}

impl<Chain: CwEnv> Shogun<Chain> {
    pub fn new(chain: Chain) -> Self {
        let staking = Staking::new("shogun:staking", chain.clone());
        let bond = Bond::new("shogun:bond", chain.clone());
        let oracle = Oracle::new("shogun:oracle", chain.clone());

        Self {
            staking,
            bonds: HashMap::from_iter([(BOND_CODE_ID.to_string(), bond)]),
            oracle,
        }
    }

    pub fn instantiate(
        &self,
        deploy_data: ShogunDeployment,
    ) -> Result<(), <Self as Deploy<Chain>>::Error> {
        let chain = self.oracle.get_chain();
        let sender = chain.sender().to_string();
        self.oracle.instantiate(
            &oracle::msg::InstantiateMsg {
                owner: sender.clone(),
                base_asset: deploy_data.base_asset,
            },
            None,
            None,
        )?;

        self.staking.instantiate(
            &staking::msg::InstantiateMsg {
                admin: Some(sender),
                epoch_length: deploy_data.epoch_length,
                first_epoch_time: deploy_data.first_epoch_time,
                epoch_apr: deploy_data.epoch_apr,
                initial_balances: deploy_data
                    .initial_balances
                    .into_iter()
                    .map(|(recipient, amount)| (recipient, amount.into()))
                    .collect(),
            },
            None,
            Some(&coins(
                deploy_data.amount_to_create_denom * 2,
                deploy_data.fee_token,
            )),
        )?;

        Ok(())
    }

    pub fn bond_code_id(&self) -> Result<u64, <Self as Deploy<Chain>>::Error> {
        self.bonds.get(BOND_CODE_ID).unwrap().code_id()
    }

    pub fn add_bond(
        &mut self,
        chain: Chain,
        config: BondConfig,
    ) -> Result<(), <Self as Deploy<Chain>>::Error> {
        // Register the price feed
        self.oracle
            .register_feeder(config.bond_token_denom.clone(), chain.sender().to_string())?;

        // Instantiate the bond on a new contract
        let bond = Bond::new(
            Self::bond_contract_name(config.bond_token_denom.clone()),
            chain.clone(),
        );
        bond.set_code_id(self.bond_code_id()?);
        bond.instantiate(
            &bond::msg::InstantiateMsg {
                admin: Some(chain.sender().to_string()),
                oracle: self.oracle.address()?.to_string(),
                oracle_trust_period: config.oracle_trust_period,
                principle: config.bond_token_denom.clone(),
                staking: self.staking.address()?.to_string(),
                usd: self.oracle.config()?.base_asset,
                treasury: config.treasury,
                terms: config.terms,
            },
            None,
            None,
        )?;

        self.staking.update_config(
            Some(vec![BondContractsElem {
                bond_token: config.bond_token_denom.clone(),
                bond_address: bond.address()?.to_string(),
            }]),
            None,
            None,
            None,
            None,
        )?;

        self.bonds.insert(config.bond_token_denom, bond);

        Ok(())
    }

    pub fn bond_contract_name(denom: String) -> String {
        format!("shogun:bond-{}", denom)
    }
}

#[cw_serde]
pub struct ShogunDeployment {
    pub base_asset: String,
    pub epoch_length: u64,
    pub first_epoch_time: u64,
    pub epoch_apr: Decimal256,
    pub initial_balances: Vec<(String, u128)>,
    pub amount_to_create_denom: u128,
    pub fee_token: String,
}

#[cw_serde]
pub struct BondConfig {
    pub bond_token_denom: String,
    pub treasury: String,
    pub oracle_trust_period: u64,
    pub terms: Terms,
}
