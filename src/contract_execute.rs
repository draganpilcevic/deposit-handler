use cosmwasm_std::{to_binary, Coin, DepsMut, Env, Event, MessageInfo, Response, Uint128, WasmMsg};

use crate::{
    msg::{ExecuteMsg, ExternalExecuteMsg},
    state::{BOND_STATUS_TRACKER, CONFIG, ID_TO_ADDRESS_TRACKER},
    typing::{BondStatus, UnbondingElement},
    ContractError,
};

pub fn route_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond { id } => bond(deps, info, id),
        ExecuteMsg::StartUnbond { id, share_amount } => {
            start_unbond(deps, env, info, id, share_amount)
        }
        ExecuteMsg::Unbond { id, share_amount } => unbond(deps, env, info, id, share_amount),

        _ => return Err(ContractError::Never {}),
    }
}

fn bond(deps: DepsMut, info: MessageInfo, id: String) -> Result<Response, ContractError> {
    // start by checking if deposits are valid
    if info.funds.len() != 2 {
        return Err(ContractError::MismatchAmountDenoms {
            req_amount_denoms: 2,
        });
    } else if info.funds[0].amount != info.funds[1].amount {
        return Err(ContractError::FundsAmountNotEqual {});
    } else if info.funds[0].denom == info.funds[1].denom {
        return Err(ContractError::FundsDenomAreSame {});
    }

    // get config to check if denoms sent are allowed
    let config = CONFIG.load(deps.storage)?;
    if !config.is_valid_denom(&info.funds[0].denom) {
        return Err(ContractError::InvalidDenom {
            denom: info.funds[0].denom.to_owned(),
        });
    } else if !config.is_valid_denom(&info.funds[1].denom) {
        return Err(ContractError::InvalidDenom {
            denom: info.funds[1].denom.to_owned(),
        });
    }

    // check if the ID is available, or if caller is owner of the id
    match ID_TO_ADDRESS_TRACKER.load(deps.storage, id.clone()) {
        Ok(owner) => {
            if owner != info.sender {
                return Err(ContractError::Unauthorized {});
            }
        }
        Err(_) => {
            // register id to caller
            ID_TO_ADDRESS_TRACKER.save(deps.storage, id.clone(), &info.sender)?;
        }
    }

    BOND_STATUS_TRACKER.update(
        deps.storage,
        id.clone(),
        |bond_status| -> Result<BondStatus, ContractError> {
            let mut bond_status = bond_status.unwrap_or_default();
            bond_status.sent_to_bond.denom_1 += info.funds[0].amount;
            bond_status.sent_to_bond.denom_2 += info.funds[0].amount;

            return Ok(bond_status);
        },
    )?;

    // deposit has been written to storage, now can create the funds messages towards the routers
    let msg_router_1 = WasmMsg::Execute {
        contract_addr: config.router_denom_1.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnBond { id: id.to_owned() })?,
        funds: vec![Coin {
            denom: config.denom_1,
            amount: info.funds[0].amount,
        }],
    };

    let msg_router_2 = WasmMsg::Execute {
        contract_addr: config.router_denom_2.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnBond { id: id.to_owned() })?,
        funds: vec![Coin {
            denom: config.denom_2,
            amount: info.funds[0].amount,
        }],
    };

    // send the messages and emit an event
    return Ok(Response::new()
        .add_message(msg_router_1)
        .add_message(msg_router_2)
        .add_event(
            Event::new("bond")
                .add_attribute("method", "bond")
                .add_attribute("caller", info.sender)
                .add_attribute("id", id)
                .add_attribute("value", info.funds[0].amount),
        ));
}

fn start_unbond(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: String,
    share_amount: Uint128,
) -> Result<Response, ContractError> {
    // check if caller is owner of id
    match ID_TO_ADDRESS_TRACKER.load(deps.storage, id.clone()) {
        Err(_) => return Err(ContractError::IdNotAllocated {}),
        Ok(owner) => {
            if owner != info.sender {
                return Err(ContractError::Unauthorized {});
            }
        }
    };

    // track funds as awaiting confirmation for start of unbonding
    // only allow a single unconfirmed unbonding at a time
    match BOND_STATUS_TRACKER.update(
        deps.storage,
        id.clone(), // info.sender.clone()),
        |bond_status_data| -> Result<BondStatus, ContractError> {
            let mut bond_status = bond_status_data.unwrap_or_default();

            // prevent unbonding if there are unconfirmed unbonds
            if bond_status.unconfirmed_unbonding.denom_1 != Uint128::zero()
                || bond_status.unconfirmed_unbonding.denom_2 != Uint128::zero()
            {
                return Err(ContractError::NoStartUnbondingIfExistingUnconfirmed {});
            }

            // prevent unbonding if share amount is too high
            // bonded should be same since logic is 1:1, but we'll stay safe
            if share_amount > bond_status.bonded.denom_1
                || share_amount > bond_status.bonded.denom_2
            {
                return Err(ContractError::StartUnbondAmountTooHigh {});
            }

            bond_status.bonded.denom_1 -= share_amount;
            bond_status.bonded.denom_2 -= share_amount;

            bond_status.unconfirmed_unbonding.denom_1 += share_amount;
            bond_status.unconfirmed_unbonding.denom_2 += share_amount;

            return Ok(bond_status);
        },
    ) {
        Err(e) => return Err(e),
        Ok(_) => (),
    };

    // send messages to the relayers
    let config = CONFIG.load(deps.storage)?;

    let msg_router_1 = WasmMsg::Execute {
        contract_addr: config.router_denom_1.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnStartUnbond {
            id: id.clone(),
            share_amount: share_amount,
        })?,
        funds: vec![],
    };

    let msg_router_2 = WasmMsg::Execute {
        contract_addr: config.router_denom_2.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnStartUnbond {
            id: id.clone(),
            share_amount: share_amount,
        })?,
        funds: vec![],
    };

    return Ok(Response::new()
        .add_message(msg_router_1)
        .add_message(msg_router_2)
        .add_event(
            Event::new("start_unbond")
                .add_attribute("method", "start_unbond")
                .add_attribute("caller", info.sender)
                .add_attribute("id", id)
                .add_attribute("share_amount", share_amount),
        ));
}

fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    share_amount: Uint128,
) -> Result<Response, ContractError> {
    // check if caller is owner of id
    match ID_TO_ADDRESS_TRACKER.load(deps.storage, id.clone()) {
        Err(_) => return Err(ContractError::IdNotAllocated {}),
        Ok(owner) => {
            if owner != info.sender {
                return Err(ContractError::Unauthorized {});
            }
        }
    };

    // load config
    let config = CONFIG.load(deps.storage)?;

    // now consume in unbonding and set in unconfirmed_unbonded
    match BOND_STATUS_TRACKER.update(
        deps.storage,
        id.clone(),
        |bond_status_data| -> Result<BondStatus, ContractError> {
            let mut bond_status = bond_status_data.unwrap_or_default();

            // set as sent for unbond, keeping track of intermediary state if there is a problem with the routers
            bond_status.sent_for_unbond.denom_1 += share_amount;
            bond_status.sent_for_unbond.denom_2 += share_amount;

            // now consume unbonding elements
            // we'll iterate and pop
            let mut to_consume = (share_amount, share_amount);
            let mut kept_elements: Vec<UnbondingElement> = vec![];
            while to_consume.0 > Uint128::zero() || to_consume.1 > Uint128::zero() {
                let mut elem = match bond_status.unbonding.pop() {
                    Some(val) => val,
                    // if no more elements, this means there is not enough unbonded assets to honor the call
                    None => return Err(ContractError::UnbondAmountTooHigh {}),
                };

                if elem.denom == config.denom_1
                    && elem
                        .unbonding_start_time
                        .plus_seconds(config.lock_period_denom_1)
                        < env.block.time
                {
                    if elem.value > to_consume.0 {
                        // elem has more value than what's left to consume
                        // so partial consume and push back
                        elem.value = elem.value - to_consume.0;
                        kept_elements.push(elem);
                        to_consume.0 = Uint128::zero();
                    } else {
                        // consume entirely, discard the element
                        to_consume.0 -= elem.value;
                    }
                } else if elem.denom == config.denom_2
                    && elem
                        .unbonding_start_time
                        .plus_seconds(config.lock_period_denom_2)
                        < env.block.time
                {
                    if elem.value > to_consume.1 {
                        // elem has more value than what's left to consume
                        // so partial consume and push back
                        elem.value = elem.value - to_consume.1;
                        kept_elements.push(elem);
                        to_consume.1 = Uint128::zero();
                    } else {
                        // consume entirely, discard the element
                        to_consume.1 -= elem.value;
                    }
                } else {
                    kept_elements.push(elem);
                }
            }

            // push elements back into unbonding
            bond_status.unbonding.append(&mut kept_elements);
            return Ok(bond_status);
        },
    ) {
        Err(e) => return Err(e),
        Ok(_) => (),
    };

    // now send messages to router to get the assets back
    let msg_router_1 = WasmMsg::Execute {
        contract_addr: config.router_denom_1.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnUnbond {
            id: id.clone(),
            share_amount: share_amount,
        })?,
        funds: vec![],
    };

    let msg_router_2 = WasmMsg::Execute {
        contract_addr: config.router_denom_2.into_string(),
        msg: to_binary(&ExternalExecuteMsg::OnUnbond {
            id: id.clone(),
            share_amount: share_amount,
        })?,
        funds: vec![],
    };

    return Ok(Response::new()
        .add_message(msg_router_1)
        .add_message(msg_router_2)
        .add_event(
            Event::new("unbond")
                .add_attribute("method", "unbond")
                .add_attribute("caller", info.sender)
                .add_attribute("id", id)
                .add_attribute("share_amount", share_amount),
        ));
}
