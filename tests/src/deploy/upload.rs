use std::collections::HashMap;

use cw_orch::environment::CwEnv;

pub struct Shogun<Chain: CwEnv> {
    pub staking: Staking<Chain>,
    pub bonds: HashMap<String, Bond<Chain>>,
    pub oracle: Oracle<Chain>,
}

pub fn upload(chain: CwEnv) -> anyhow::Result<OHM> {}
