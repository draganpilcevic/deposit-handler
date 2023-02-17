use cosmwasm_std::{Addr, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    /// The lock period of the strategy for denom 1
    pub lock_period_denom_1: u64,
    /// The lock period of the strategy for denom 2
    pub lock_period_denom_2: u64,
    /// The first allowed denom for bonding
    pub denom_1: String,
    /// Router to stake denom_1
    pub router_denom_1: Addr,
    /// The second allowed denom for bonding
    pub denom_2: String,
    /// Router to stake denom_1
    pub router_denom_2: Addr,
}

impl Config {
    pub fn is_valid_denom(&self, denom: &str) -> bool {
        return self.denom_1 == denom || self.denom_2 == denom;
    }

    pub fn is_valid_callback_caller(&self, caller: &Addr) -> bool {
        return self.router_denom_1.eq(caller) || self.router_denom_2.eq(caller);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BondStatusData {
    pub denom_1: Uint128,
    pub denom_2: Uint128,
}


impl BondStatusData {
    /// Create a new BondStatusData from a single value.
    /// To be used for a new sent_to_bond element in BondStatus since funds are presumed to be equal
    pub fn new(value: Uint128) -> Self {
        return BondStatusData {
            denom_1: value,
            denom_2: value,
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct UnbondingElement {
    pub denom: String,
    pub value: Uint128,
    pub unbonding_start_time: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BondStatus {
    pub sent_to_bond: BondStatusData,
    pub bonded: BondStatusData,
    pub unconfirmed_unbonding: BondStatusData,
    pub unbonding: Vec<UnbondingElement>,
    pub sent_for_unbond: BondStatusData,
}
