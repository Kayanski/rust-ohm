use std::{collections::HashMap, path::PathBuf};

use bond::interface::Bond;
use bond::state::Terms;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coins, Addr, Decimal256};
use cw_orch::{
    contract::interface_traits::{ContractInstance, CwOrchInstantiate, CwOrchUpload},
    deploy::Deploy,
    environment::CwEnv,
    prelude::CwOrchError,
};
use staking_contract::interface::Staking;
use staking_contract::msg::BondContractsElem;
use staking_contract::msg::ExecuteMsgFns as _;
use staking_contract::msg::QueryMsgFns as _;
use staking_token::interface::StakingToken;

pub const BOND_CODE_ID: &str = "bond-code-id";

#[derive(Clone)]
pub struct Shogun<Chain: CwEnv> {
    pub staking: Staking<Chain>,
    pub bonds: HashMap<String, Bond<Chain>>,
    pub staking_token: StakingToken<Chain>,
}
impl<Chain: CwEnv> Deploy<Chain> for Shogun<Chain> {
    type Error = CwOrchError;

    type DeployData = ShogunDeployment;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let shogun = Self::new(chain);

        shogun.staking.upload()?;
        shogun.bonds.get(BOND_CODE_ID).unwrap().upload()?;
        shogun.staking_token.upload()?;

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
        let token_box: Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>> =
            Box::new(&mut self.staking_token);
        self.bonds
            .iter_mut()
            .map(|(_, d)| {
                let boxed: Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>> = Box::new(d);
                boxed
            })
            .chain([staking_box, token_box])
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
        let staking_token = StakingToken::new("shogun:staking-token", chain.clone());

        Self {
            staking,
            bonds: HashMap::from_iter([(BOND_CODE_ID.to_string(), bond)]),
            staking_token,
        }
    }

    pub fn instantiate(
        &self,
        deploy_data: ShogunDeployment,
    ) -> Result<(), <Self as Deploy<Chain>>::Error> {
        let chain = self.staking.get_chain();
        let sender = chain.sender().to_string();

        self.staking.instantiate(
            &staking_contract::msg::InstantiateMsg {
                admin: Some(sender),
                epoch_length: deploy_data.epoch_length,
                first_epoch_time: deploy_data.first_epoch_time,
                epoch_apr: deploy_data.epoch_apr,
                initial_balances: deploy_data
                    .initial_balances
                    .into_iter()
                    .map(|(recipient, amount)| (recipient, amount.into()))
                    .collect(),
                warmup_length: deploy_data.warmup_length,
            },
            None,
            Some(&coins(
                deploy_data.amount_to_create_denom * 2,
                deploy_data.fee_token,
            )),
        )?;

        self.staking.instantiate_contracts(
            deploy_data.cw1_code_id,
            deploy_data.staking_symbol,
            deploy_data.staking_name,
            self.staking_token.code_id()?,
        )?;

        let config = self.staking.config()?;
        self.staking_token
            .set_address(&Addr::unchecked(config.sohm_address));
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
        // Instantiate the bond on a new contract
        let bond = Bond::new(
            Self::bond_contract_name(config.bond_token_denom.clone()),
            chain.clone(),
        );
        bond.set_code_id(self.bond_code_id()?);
        bond.instantiate(
            &bond::msg::InstantiateMsg {
                admin: Some(chain.sender().to_string()),
                principle: config.bond_token_denom.clone(),
                staking: self.staking.address()?.to_string(),
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
    pub epoch_length: u64,
    pub first_epoch_time: u64,
    pub epoch_apr: Decimal256,
    pub initial_balances: Vec<(String, u128)>,
    pub amount_to_create_denom: u128,
    pub fee_token: String,
    pub staking_symbol: String,
    pub staking_name: String,
    pub warmup_length: u64,
    pub cw1_code_id: u64,
}

#[cw_serde]
pub struct BondConfig {
    pub bond_token_denom: String,
    pub treasury: String,
    pub terms: Terms,
}
