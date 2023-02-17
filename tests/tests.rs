#[cfg(test)]
mod test {
    use cosmwasm_std::coin;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::mock_dependencies_with_balances;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::Addr;
    use cosmwasm_std::Deps;
    use cosmwasm_std::Empty;
    use cosmwasm_std::Env;
    use cosmwasm_std::MemoryStorage;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::Uint128;
    use deposit_handler::contract::execute;
    use deposit_handler::contract::instantiate;
    use deposit_handler::contract::query;
    use deposit_handler::msg::BondResponse;
    use deposit_handler::msg::Callback;
    use deposit_handler::msg::ExecuteMsg;
    use deposit_handler::msg::GetBondStatusResponse;
    use deposit_handler::msg::GetSharesAvailableUnbondResponse;
    use deposit_handler::msg::InstantiateMsg;
    use deposit_handler::msg::QueryMsg;
    use deposit_handler::msg::StartUnbondResponse;
    use deposit_handler::msg::UnbondResponse;
    use deposit_handler::typing::BondStatus;
    use deposit_handler::typing::Config;
    use deposit_handler::ContractError;

    const DENOM_1: &'static str = "qusd";
    const ROUTER_DENOM_1: &'static str = "router_qusd";
    const DENOM_2: &'static str = "uatom";
    const ROUTER_DENOM_2: &'static str = "router_uatom";

    const LOCK_PERIOD_DENOM_1: u64 = 1000;
    const LOCK_PERIOD_DENOM_2: u64 = 1000;

    const INITIAL_BALANCE: u128 = 100_000;

    const _ADMIN: &'static str = "admin";
    const USER: &'static str = "user";

    fn setup(config: Config) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier, Empty>, Env) {
        let mut deps = mock_dependencies_with_balances(&[(
            USER,
            &[
                coin(INITIAL_BALANCE, DENOM_1),
                coin(INITIAL_BALANCE, DENOM_2),
            ],
        )]);
        let env = mock_env();

        // create contract
        let msg = InstantiateMsg { config: config };
        instantiate(deps.as_mut(), env.to_owned(), mock_info("sender", &[]), msg).unwrap();

        return (deps, env);
    }

    fn get_test_config() -> Config {
        return Config {
            lock_period_denom_1: LOCK_PERIOD_DENOM_1,
            lock_period_denom_2: LOCK_PERIOD_DENOM_2,
            denom_1: DENOM_1.to_owned(),
            router_denom_1: Addr::unchecked(ROUTER_DENOM_1),
            denom_2: DENOM_2.to_owned(),
            router_denom_2: Addr::unchecked(ROUTER_DENOM_2),
        };
    }

    fn get_bond_status(deps: &Deps, env: &Env) -> BondStatus {
        let msg = QueryMsg::GetBondStatus {
            id: "test_id".into(),
        };
        let res = query(deps.to_owned(), env.to_owned(), msg).unwrap();
        let res: GetBondStatusResponse = from_binary(&res).unwrap();

        return res.bond_status.unwrap();
    }

    #[test]
    /// Test if no problem when instantiating the contract
    fn successful_instantiation() {
        setup(get_test_config());
    }

    #[test]
    /// Test bonding related errors
    fn bonding_errors() {
        let (mut deps, env) = setup(get_test_config());

        let msg = ExecuteMsg::Bond {
            id: "test_id".into(),
        };

        // no funds attached
        let msg_info = mock_info(USER, &[]);
        let res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap_err();

        assert_eq!(
            res,
            ContractError::MismatchAmountDenoms {
                req_amount_denoms: 2
            }
        );

        // only one fund attached
        let msg_info = mock_info(USER, &[coin(10000, DENOM_1)]);
        let res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap_err();

        assert_eq!(
            res,
            ContractError::MismatchAmountDenoms {
                req_amount_denoms: 2
            }
        );

        // funds values mismatch
        let msg_info = mock_info(USER, &[coin(10000, DENOM_1), coin(5555, DENOM_2)]);
        let res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap_err();

        assert_eq!(res, ContractError::FundsAmountNotEqual {});

        // send valid denoms required
        let msg_info = mock_info(
            USER,
            &[coin(10000, DENOM_1), coin(10000, "not_the_right_denom")],
        );
        let res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap_err();

        assert_eq!(
            res,
            ContractError::InvalidDenom {
                denom: "not_the_right_denom".into()
            }
        );
    }

    #[test]
    /// Testing a full run of the contract: bonding then unbonding
    fn successful_run() {
        let (mut deps, mut env) = setup(get_test_config());

        // try bonding assets
        let msg = ExecuteMsg::Bond {
            id: "test_id".into(),
        };
        let msg_info = mock_info(USER, &[coin(10_000, DENOM_1), coin(10_000, DENOM_2)]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // check state, should have 10k sent to bond
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            bond_status.sent_to_bond.denom_1 == bond_status.sent_to_bond.denom_2
                && bond_status.sent_to_bond.denom_2 == Uint128::new(10000)
        );

        // assets should be bonded. Send mock callbacks from associated contracts
        let callback = Callback::BondResponse(BondResponse {
            share_amount: Uint128::from(10_000u128),
            bond_id: "test_id".to_string(),
        });
        let msg = ExecuteMsg::Callback(callback);

        // from contract handling first denom send callback
        let msg_info = mock_info(ROUTER_DENOM_1, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // check state, should have 10k bonded for denom 1 and 10k sent to bond for denom 2
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            bond_status.sent_to_bond.denom_1 == Uint128::zero()
                && bond_status.sent_to_bond.denom_2 == Uint128::new(10000)
                && bond_status.bonded.denom_1 == Uint128::new(10000)
        );

        // from contract handling second denom
        let msg_info = mock_info(ROUTER_DENOM_2, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // again check state, should have 10k bonded for denom 1 and denom 2
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            bond_status.sent_to_bond.denom_1 == Uint128::zero()
                && bond_status.sent_to_bond.denom_2 == Uint128::zero()
                && bond_status.bonded.denom_1 == Uint128::new(10000)
                && bond_status.bonded.denom_2 == Uint128::new(10000)
        );

        // now start unbonding
        let msg = ExecuteMsg::StartUnbond {
            id: "test_id".into(),
            share_amount: Uint128::from(500u128),
        };
        let msg_info = mock_info(USER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info.clone(), msg.clone()).unwrap();

        // checking state, we should have 500 in unconfirmed unbonding
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            bond_status.unconfirmed_unbonding.denom_1 == Uint128::new(500)
                && bond_status.unconfirmed_unbonding.denom_1 == Uint128::new(500)
                && bond_status.bonded.denom_1 == Uint128::new(10000 - 500)
                && bond_status.bonded.denom_2 == Uint128::new(10000 - 500)
        );

        // we can only process one start unbond at a time. New execute should return an error
        let msg = ExecuteMsg::StartUnbond {
            id: "test_id".into(),
            share_amount: Uint128::from(500u128),
        };
        let msg_info = mock_info(USER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info.clone(), msg.clone()).unwrap_err();

        // send callbacks
        let callback = Callback::StartUnbondResponse(StartUnbondResponse {
            unbond_id: "test_id".into(),
        });
        let msg = ExecuteMsg::Callback(callback);

        let msg_info = mock_info(ROUTER_DENOM_1, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        let msg_info = mock_info(ROUTER_DENOM_2, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // checking state, we should have 2 elements of value 500 in unbonding
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            bond_status.unbonding.len() == 2
                && bond_status.unbonding[0].value == Uint128::new(500)
                && bond_status.unbonding[1].value == Uint128::new(500)
        );

        // total available for unbond operation should be 0 since unlock time has not been reached
        let msg = QueryMsg::GetSharesAvailableUnbond {
            id: "test_id".into(),
        };
        let res: GetSharesAvailableUnbondResponse =
            from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
        assert!(res.shares_available_unbond == Uint128::zero());

        // advance time by 1 day to enable unbonding
        env.block.time = env.block.time.plus_seconds(60 * 60 * 24);

        // now total available for unbond operation should be 500 since unlock time has been reached
        let msg = QueryMsg::GetSharesAvailableUnbond {
            id: "test_id".into(),
        };
        let res: GetSharesAvailableUnbondResponse =
            from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
        assert!(res.shares_available_unbond == Uint128::new(500));

        // trying to unbond > 500 should return an error
        let msg = ExecuteMsg::Unbond {
            id: "test_id".into(),
            share_amount: Uint128::new(50000),
        };

        let msg_info = mock_info(USER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap_err();

        // finish unbonding and get the tokens back
        let msg = ExecuteMsg::Unbond {
            id: "test_id".into(),
            share_amount: Uint128::from(250u128),
        };

        let msg_info = mock_info(USER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // check status
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            // the whole unbondings should not have been consumed
            bond_status.unbonding.len() == 2
            && bond_status.unbonding[0].value == Uint128::new(250)
            // we should have 250 shares unconfirmed for sent for unbond (so waiting for transfer)
            && bond_status.sent_for_unbond.denom_1 == Uint128::new(250)
            && bond_status.sent_for_unbond.denom_2 == Uint128::new(250)
        );

        // final callbacks sending funds back to the user
        let callback = Callback::UnbondResponse(UnbondResponse {
            unbond_id: "test_id".into(),
        });
        let msg = ExecuteMsg::Callback(callback);

        let msg_info = mock_info(ROUTER_DENOM_1, &[coin(250, DENOM_1)]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        let msg_info = mock_info(ROUTER_DENOM_2, &[coin(250, DENOM_2)]);
        let _res = execute(deps.as_mut(), env.clone(), msg_info, msg.clone()).unwrap();

        // Final check status, we'll make it a global check
        // we should have 0 in sent for unbond, 9500 in bonded, two 250 elements in unbonding
        let bond_status = get_bond_status(&deps.as_ref(), &env);
        assert!(
            // the whole unbondings should not have been consumed
            bond_status.unbonding.len() == 2
            && bond_status.unbonding[0].value == Uint128::new(250)
            && bond_status.unbonding[1].value == Uint128::new(250)

            // the bonded balances should be at 9500
            && bond_status.bonded.denom_1 == Uint128::new(9500)
            && bond_status.bonded.denom_2 == Uint128::new(9500)
        );
    }
}
