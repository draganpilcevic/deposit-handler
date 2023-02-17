use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::typing::{BondStatus, Config};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Callback {
    BondResponse(BondResponse),
    StartUnbondResponse(StartUnbondResponse),
    UnbondResponse(UnbondResponse),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// BondResponse is the response of a the strategy once the funds are succesfully bonded
pub struct BondResponse {
    pub share_amount: Uint128,
    pub bond_id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// UnbondResponse is the response of a strategy once shares succesfully start unbonding
pub struct StartUnbondResponse {
    pub unbond_id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UnbondResponse {
    pub unbond_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Bond { id: String },
    StartUnbond { id: String, share_amount: Uint128 },
    Unbond { id: String, share_amount: Uint128 },
    Callback(Callback),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetConfigResponse)]
    GetConfig {},

    #[returns(GetBondStatusResponse)]
    GetBondStatus { id: String },

    #[returns(GetSharesAvailableUnbondResponse)]
    GetSharesAvailableUnbond { id: String },
}

#[cw_serde]
pub struct GetConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct GetBondStatusResponse {
    pub bond_status: Option<BondStatus>,
}

#[cw_serde]
pub struct GetSharesAvailableUnbondResponse {
    pub shares_available_unbond: Uint128,
}

#[cw_serde]
pub enum ExternalExecuteMsg {
    OnBond { id: String },
    OnStartUnbond { id: String, share_amount: Uint128 },
    OnUnbond { id: String, share_amount: Uint128 },
}
