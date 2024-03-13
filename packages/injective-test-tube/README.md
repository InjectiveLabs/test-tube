# injective-test-tube

CosmWasm x Injective integration testing library that, unlike `cw-multi-test`, it allows you to test your cosmwasm contract against real chain's logic instead of mocks.

As the project depends on currently private repos, no crate is published rather please refer to [`CHANGELOG`](./CHANGELOG.md) for features and update information.

## Table of Contents

- [Getting Started](#getting-started)
- [Debugging](#debugging)
- [Using Module Wrapper](#using-module-wrapper)
- [Versioning](#versioning)

## Getting Started

To demonstrate how `injective-test-tube` works, let use simple example contract: [cw-whitelist](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw1-whitelist) from `cw-plus`.

Here is how to setup the test:

```rust
use cosmwasm_std::Coin;
use injective_test_tube::InjectiveTestApp;

// create new injective appchain instance.
let app = InjectiveTestApp::new();

// create new account with initial funds
let accs = app
    .init_accounts(
        &[
            Coin::new(1_000_000_000_000, "usdt"),
            Coin::new(1_000_000_000_000, "inj"),
        ],
        2,
    )
    .unwrap();

let admin = &accs[0];
let new_admin = &accs[1];
```

Now we have the appchain instance and accounts that have some initial balances and can interact with the appchain.
This does not run Docker instance or spawning external process, it just loads the appchain's code as a library to create an in memory instance.

Note that `init_accounts` is a convenience function that creates multiple accounts with the same initial balance.
If you want to create just one account, you can use `init_account` instead.

```rust
use cosmwasm_std::Coin;
use injective_test_tube::InjectiveTestApp;

let app = InjectiveTestApp::new();

let account = app.init_account(&[
    Coin::new(1_000_000_000_000, "usdt"),
    Coin::new(1_000_000_000_000, "inj"),
]);
```

Now if we want to test a cosmwasm contract, we need to

- build the wasm file
- store code
- instantiate

Then we can start interacting with our contract. Let's do just that.

```rust
use cosmwasm_std::Coin;
use cw1_whitelist::msg::{InstantiateMsg}; // for instantiating cw1_whitelist contract
use injective_test_tube::{Account, Module, InjectiveTestApp, Wasm};

let app = InjectiveTestApp::new();
let accs = app
    .init_accounts(
        &[
            Coin::new(1_000_000_000_000, "usdt"),
            Coin::new(1_000_000_000_000, "inj"),
        ],
        2,
    )
    .unwrap();
let admin = &accs[0];
let new_admin = &accs[1];

// ============= NEW CODE ================

// `Wasm` is the module we use to interact with cosmwasm related logic on the appchain
// it implements `Module` trait which you will see more later.
let wasm = Wasm::new(&app);

// Load compiled wasm bytecode
let wasm_byte_code = std::fs::read("./test_artifacts/cw1_whitelist.wasm").unwrap();
let code_id = wasm
    .store_code(&wasm_byte_code, None, admin)
    .unwrap()
    .data
    .code_id;
```

Not that in this example, it loads wasm bytecode from [cw-plus release](https://github.com/CosmWasm/cw-plus/releases) for simple demonstration purposes.
You might want to run `cargo wasm` and find your wasm file in `target/wasm32-unknown-unknown/release/<contract_name>.wasm`.

```rust
use cosmwasm_std::Coin;
use cw1_whitelist::msg::{InstantiateMsg, QueryMsg, AdminListResponse};
use injective_test_tube::{Account, Module, InjectiveTestApp, Wasm};

let app = InjectiveTestApp::new();
let accs = app
    .init_accounts(
        &[
            Coin::new(1_000_000_000_000, "usdt"),
            Coin::new(1_000_000_000_000, "inj"),
        ],
        2,
    )
    .unwrap();
let admin = &accs[0];
let new_admin = &accs[1];

let wasm = Wasm::new(&app);


let wasm_byte_code = std::fs::read("./test_artifacts/cw1_whitelist.wasm").unwrap();
let code_id = wasm
    .store_code(&wasm_byte_code, None, admin)
    .unwrap()
    .data
    .code_id;

// ============= NEW CODE ================

// instantiate contract with initial admin and make admin list mutable
let init_admins = vec![admin.address()];
let contract_addr = wasm
    .instantiate(
        code_id,
        &InstantiateMsg {
            admins: init_admins.clone(),
            mutable: true,
        },
        None, // contract admin used for migration, not the same as cw1_whitelist admin
        Some("Test label"), // contract label
        &[], // funds
        admin, // signer
    )
    .unwrap()
    .data
    .address;

// query contract state to check if contract instantiation works properly
let admin_list = wasm
    .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    .unwrap();

assert_eq!(admin_list.admins, init_admins);
assert!(admin_list.mutable);
```

Now let's execute the contract and verify that the contract's state is updated properly.

```rust
use cosmwasm_std::Coin;
use cw1_whitelist::msg::{InstantiateMsg, QueryMsg, ExecuteMsg, AdminListResponse};
use injective_test_tube::{Account, Module, InjectiveTestApp, Wasm};

let app = InjectiveTestApp::new();
let accs = app
    .init_accounts(
        &[
            Coin::new(1_000_000_000_000, "usdt"),
            Coin::new(1_000_000_000_000, "inj"),
        ],
        2,
    )
    .unwrap();
let admin = &accs[0];
let new_admin = &accs[1];

let wasm = Wasm::new(&app);


let wasm_byte_code = std::fs::read("./test_artifacts/cw1_whitelist.wasm").unwrap();
let code_id = wasm
    .store_code(&wasm_byte_code, None, admin)
    .unwrap()
    .data
    .code_id;

// instantiate contract with initial admin and make admin list mutable
let init_admins = vec![admin.address()];
let contract_addr = wasm
    .instantiate(
        code_id,
        &InstantiateMsg {
            admins: init_admins.clone(),
            mutable: true,
        },
        None, // contract admin used for migration, not the same as cw1_whitelist admin
        Some("Test label"), // contract label
        &[], // funds
        admin, // signer
    )
    .unwrap()
    .data
    .address;

let admin_list = wasm
    .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    .unwrap();

assert_eq!(admin_list.admins, init_admins);
assert!(admin_list.mutable);

// ============= NEW CODE ================

// update admin list and rechec the state
let new_admins = vec![new_admin.address()];
wasm.execute::<ExecuteMsg>(
    &contract_addr,
    &ExecuteMsg::UpdateAdmins {
        admins: new_admins.clone(),
    },
    &[],
    admin,
)
.unwrap();

let admin_list = wasm
    .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
    .unwrap();

assert_eq!(admin_list.admins, new_admins);
assert!(admin_list.mutable);
```

## Debugging

In your contract code, if you want to debug, you can use [`deps.api.debug(..)`](https://docs.rs/cosmwasm-std/latest/cosmwasm_std/trait.Api.html#tymethod.debug) which will print the debug message to stdout. `wasmd` disabled this by default but `InjectiveTestApp` allows stdout emission so that you can debug your smart contract while running tests.

## Using Module Wrapper

In some cases, you might want to interact directly with appchain logic to setup the environment or query appchain's state.
Module wrappers provides convenient functions to interact with the appchain's module.

Let's try to interact with `Exchange` module:

```rust
use cosmwasm_std::{Addr, Coin};
use injective_std::types::injective::exchange::v1beta1::{
    MarketStatus, MsgInstantSpotMarketLaunch,
    QuerySpotMarketsRequest, QuerySpotMarketsResponse, SpotMarket,
};
use injective_test_tube::{Account, Exchange, InjectiveTestApp};
use test_tube_inj::Module;

let app = InjectiveTestApp::new();
let signer = app
    .init_account(&[
        Coin::new(10_000_000_000_000_000_000_000u128, "inj"),
        Coin::new(100_000_000_000_000_000_000u128, "usdt"),
    ])
    .unwrap();
let trader = app
    .init_account(&[
        Coin::new(10_000_000_000_000_000_000_000u128, "inj"),
        Coin::new(100_000_000_000_000_000_000u128, "usdt"),
    ])
    .unwrap();
let exchange = Exchange::new(&app);

exchange
    .instant_spot_market_launch(
        MsgInstantSpotMarketLaunch {
            sender: signer.address(),
            ticker: "INJ/USDT".to_owned(),
            base_denom: "inj".to_owned(),
            quote_denom: "usdt".to_owned(),
            min_price_tick_size: "10000".to_owned(),
            min_quantity_tick_size: "100000".to_owned(),
        },
        &signer,
    )
    .unwrap();

exchange
    .instant_spot_market_launch(
        MsgInstantSpotMarketLaunch {
            sender: signer.address(),
            ticker: "INJ/USDT".to_owned(),
            base_denom: "inj".to_owned(),
            quote_denom: "usdt".to_owned(),
            min_price_tick_size: "10000".to_owned(),
            min_quantity_tick_size: "100000".to_owned(),
        },
        &signer,
    )
    .unwrap_err();

app.increase_time(1u64);

let spot_markets = exchange
    .query_spot_markets(&QuerySpotMarketsRequest {
        status: "Active".to_owned(),
        market_ids: vec![],
    })
    .unwrap();

let expected_response = QuerySpotMarketsResponse {
    markets: vec![SpotMarket {
        ticker: "INJ/USDT".to_string(),
        base_denom: "inj".to_string(),
        quote_denom: "usdt".to_string(),
        maker_fee_rate: "-100000000000000".to_string(),
        taker_fee_rate: "1000000000000000".to_string(),
        relayer_fee_share_rate: "400000000000000000".to_string(),
        market_id: "0xd5a22be807011d5e42d5b77da3f417e22676efae494109cd01c242ad46630115"
            .to_string(),
        status: MarketStatus::Active.into(),
        min_price_tick_size: "10000".to_string(),
        min_quantity_tick_size: "100000".to_string(),
    }],
};
assert_eq!(spot_markets, expected_response);
```

Additional examples can be found in the [modules](./src/module/) directory.

## Versioning

The version of injective-test-tube is determined by the versions of its dependencies, injective and test-tube, as well as its own changes. The version is represented in the format A.B.C, where:

- A is the major version of injective,
- B is the minor version of test-tube,
- C is the patch number of injective-test-tube itself.

When a new version of injective is released and contains breaking changes, we will also release breaking changes from test-tube if any and increment the major version of injective-test-tube. This way, it's clear that the new version of injective-test-tube is not backwards-compatible with previous versions.

When adding a new feature to injective-test-tube that is backward-compatible, the minor version number will be incremented.

When fixing bugs or making other changes that are `injective-test-tube` specific and backward-compatible, the patch number will be incremented.

Please review the upgrade guide for upgrading the package, in case of breaking changes

It is important to note that we track the version of the package independent of the version of dependencies.
