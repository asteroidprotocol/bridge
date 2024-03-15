use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use asteroid_bridge::contract::instantiate;
use asteroid_bridge::execute::execute;
use asteroid_bridge::msg::InstantiateMsg;
use asteroid_bridge::query::query;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_schema::serde::Deserialize;
use cosmwasm_std::{
    from_slice, wasm_execute, Addr, Binary, Coin, CustomQuery, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, WasmMsg,
};
use cw_multi_test::{
    AppBuilder, BasicApp, Contract, ContractWrapper, Executor, FailingModule, WasmKeeper,
};
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::bindings::query::NeutronQuery;

pub type NeutronApp = BasicApp<NeutronMsg, NeutronQuery>;

fn mock_app(owner: &Addr, coins: Vec<Coin>) -> NeutronApp {
    AppBuilder::new()
        .with_custom(FailingModule::<NeutronMsg, NeutronQuery, Empty>::new())
        .with_wasm::<FailingModule<NeutronMsg, NeutronQuery, Empty>, WasmKeeper<_, _>>(
            WasmKeeper::new(),
        )
        .build(|router, _, storage| {
            // initialization moved to App construction
            router.bank.init_balance(storage, owner, coins).unwrap()
        })
}

fn bridge_contract() -> Box<dyn Contract<NeutronMsg, NeutronQuery>> {
    Box::new(ContractWrapper::new(execute, instantiate, noop_query))
}

fn noop_query<Q>(_deps: Deps<Q>, _env: Env, _msg: Empty) -> StdResult<Binary>
where
    Q: CustomQuery,
{
    Ok(Default::default())
}

#[test]
fn test_add_signer() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_app(&owner, vec![]);
    let contract_code = app.store_code(bridge_contract());

    let bridge_address = app
        .instantiate_contract(
            contract_code,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                signer_threshold: 1,
                bridge_ibc_channel: "channel-0".to_string(),
                ibc_timeout_seconds: 10,
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // TODO Add checks for blank IBC channel / unknown channel on chain
    // TODO Test for invalid IBC timeouts

    assert_eq!("test", "test");
}
