use asteroid_bridge::contract::instantiate;
use asteroid_bridge::execute::execute;
use asteroid_bridge::msg::InstantiateMsg;
use cosmwasm_std::{Addr, Binary, Coin, CustomQuery, Deps, Empty, Env, StdResult};
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
                ibc_timeout_seconds: 10,
                bridge_ibc_channel: "channel-0".to_string(),
                bridge_chain_id: "localgaia-1".to_string(),
            },
            &[],
            "Asteroid Bridge",
            None,
        )
        .unwrap();

    // TODO Add checks for blank IBC channel / unknown channel on chain
    // TODO Test for invalid IBC timeouts

    println!("Bridge address: {}", bridge_address);

    // Test add signer
    // let signer = Addr::unchecked("signer");

    assert_eq!("test", "test");
}
