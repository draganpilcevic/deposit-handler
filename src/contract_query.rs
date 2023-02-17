use cosmwasm_std::{to_binary, Binary, Deps, Env, StdResult, Uint128};
use erased_serde::Serialize;

use crate::{
    msg::{GetBondStatusResponse, GetConfigResponse, GetSharesAvailableUnbondResponse, QueryMsg},
    state::{BOND_STATUS_TRACKER, CONFIG},
};

pub fn route_query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let data = match msg {
        QueryMsg::GetConfig {} => get_config(deps),
        QueryMsg::GetBondStatus { id } => get_bond_status(deps, id),
        QueryMsg::GetSharesAvailableUnbond { id } => get_shares_available_unbond(deps, env, id),
    };

    return Ok(to_binary(&data)?);
}

fn get_config(deps: Deps) -> Box<dyn Serialize> {
    return Box::new(GetConfigResponse {
        config: CONFIG.load(deps.storage).unwrap(),
    });
}

fn get_bond_status(deps: Deps, id: String) -> Box<dyn Serialize> {
    return Box::new(GetBondStatusResponse {
        bond_status: BOND_STATUS_TRACKER.may_load(deps.storage, id).unwrap(),
    });
}

fn get_shares_available_unbond(deps: Deps, env: Env, id: String) -> Box<dyn Serialize> {
    let bond_status = match BOND_STATUS_TRACKER.load(deps.storage, id) {
        Err(_) => {
            return Box::new(GetSharesAvailableUnbondResponse {
                shares_available_unbond: Uint128::zero(),
            })
        }
        Ok(data) => data,
    };

    let config = CONFIG.load(deps.storage).unwrap();

    let available_denom_1: Uint128 = bond_status
        .unbonding
        .iter()
        .filter(|elem| {
            elem.denom == config.denom_1
                && elem
                    .unbonding_start_time
                    .plus_seconds(config.lock_period_denom_1)
                    < env.block.time
        })
        .map(|elem| elem.value)
        .sum();

    let available_denom_2: Uint128 = bond_status
        .unbonding
        .iter()
        .filter(|elem| {
            elem.denom == config.denom_2
                && elem
                    .unbonding_start_time
                    .plus_seconds(config.lock_period_denom_2)
                    < env.block.time
        })
        .map(|elem| elem.value)
        .sum();

    return Box::new(GetSharesAvailableUnbondResponse {
        shares_available_unbond: std::cmp::min(available_denom_1, available_denom_2),
    });
}
