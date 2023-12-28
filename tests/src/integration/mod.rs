pub mod deploy;
pub mod test_constants {
    pub const USD: &str = "uusd";
    pub const EPOCH_LENGTH: u64 = 1_000;
    pub const FIRST_EPOCH_TIME: u64 = 10_000;
    pub const EPOCH_APR: &str = "0.1";
    pub const INITIAL_SHOGUN_BALANCE: u128 = 50_000_000_000;

    pub mod bond_terms_1 {

        pub const BOND_TOKEN: &str = "inj";
        pub const CONTROL_VARIABLE: &str = "8";
        pub const MAX_DEBT: u128 = 100_000_000_000_000;
        pub const MAX_PAYOUT: &str = "100000000";
        pub const MINIMUM_PRICE: &str = "2";
        pub const VESTING_TERM: u64 = 3600;
        pub const ORACLE_TEST_PERIOD: u64 = 300;
    }
}
