use cosmwasm_std::{BankMsg, DepsMut, Env, Event, MessageInfo, Response, Uint128};

use crate::{
    msg::{BondResponse, Callback, StartUnbondResponse, UnbondResponse},
    state::{BOND_STATUS_TRACKER, CONFIG, ID_TO_ADDRESS_TRACKER},
    typing::{BondStatus, Config, UnbondingElement},
    ContractError,
};

pub fn route_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Callback,
) -> Result<Response, ContractError> {
    // check if caller is allowed
    let config = CONFIG.load(deps.storage)?;

    if !config.is_valid_callback_caller(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        Callback::BondResponse(response) => bond_response(deps, info, config, response),
        Callback::StartUnbondResponse(response) => {
            start_unbond_response(deps, env, info, config, response)
        }
        Callback::UnbondResponse(response) => unbond_response(deps, info, config, response),
    }
}

fn bond_response(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    response: BondResponse,
) -> Result<Response, ContractError> {
    // bonding is successful, update the state
    BOND_STATUS_TRACKER
        .update(
            deps.storage,
            response.bond_id.clone(), // info.sender.clone()),
            |bond_status_data| -> Result<BondStatus, ContractError> {
                let mut bond_status = bond_status_data.unwrap_or_default();

                if info.sender == config.router_denom_1 {
                    bond_status.sent_to_bond.denom_1 -= response.share_amount;
                    bond_status.bonded.denom_1 += response.share_amount;
                } else {
                    bond_status.sent_to_bond.denom_2 -= response.share_amount;
                    bond_status.bonded.denom_2 += response.share_amount;
                }

                return Ok(bond_status);
            },
        )
        .unwrap();

    return Ok(Response::new().add_event(
        Event::new("callback_bond")
            .add_attribute("method", "bond_response")
            .add_attribute("id", response.bond_id)
            .add_attribute(
                "denom",
                if info.sender == config.router_denom_1 {
                    config.denom_1
                } else {
                    config.denom_2
                },
            )
            .add_attribute("value", response.share_amount),
    ));
}

fn start_unbond_response(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: Config,
    response: StartUnbondResponse,
) -> Result<Response, ContractError> {
    BOND_STATUS_TRACKER
        .update(
            deps.storage,
            response.unbond_id.clone(), // info.sender.clone()),
            |bond_status_data| -> Result<BondStatus, ContractError> {
                let mut bond_status = bond_status_data.unwrap_or_default();

                if info.sender == config.router_denom_1 {
                    bond_status.unbonding.push(UnbondingElement {
                        denom: config.denom_1,
                        value: bond_status.unconfirmed_unbonding.denom_1,
                        unbonding_start_time: env.block.time,
                    });
                    bond_status.unconfirmed_unbonding.denom_1 = Uint128::zero();
                } else {
                    bond_status.unbonding.push(UnbondingElement {
                        denom: config.denom_2,
                        value: bond_status.unconfirmed_unbonding.denom_2,
                        unbonding_start_time: env.block.time,
                    });
                    bond_status.unconfirmed_unbonding.denom_2 = Uint128::zero();
                }

                return Ok(bond_status);
            },
        )
        .unwrap();

    return Ok(Response::new().add_event(
        Event::new("callback_start_unbond")
            .add_attribute("method", "start_unbond_response")
            .add_attribute("id", response.unbond_id),
    ));
}

fn unbond_response(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    response: UnbondResponse,
) -> Result<Response, ContractError> {
    // get address associated with the ID
    let target_addr = ID_TO_ADDRESS_TRACKER.load(deps.storage, response.unbond_id.clone())?;

    // mark the value as received
    BOND_STATUS_TRACKER.update(
        deps.storage,
        response.unbond_id.clone(),
        |bond_status| -> Result<BondStatus, ContractError> {
            let mut bond_status = bond_status.unwrap();

            if info.sender == config.router_denom_1 {
                bond_status.sent_for_unbond.denom_1 -= info.funds[0].amount;
            } else {
                // if info.sender == config.denom_2 {
                bond_status.sent_for_unbond.denom_2 -= info.funds[0].amount;
            }

            return Ok(bond_status);
        },
    )?;

    // and send message funds to it
    return Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: target_addr.clone().into_string(),
            amount: info.funds,
        })
        .add_event(
            Event::new("callback_unbond")
                .add_attribute("method", "unbond_response")
                .add_attribute("id", response.unbond_id)
                .add_attribute("beneficiary", target_addr),
        ));
}
