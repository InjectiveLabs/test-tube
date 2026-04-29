# injective-test-tube

CosmWasm x Injective integration testing library that, unlike `cw-multi-test`, it allows you to test your cosmwasm contract against real chain's logic instead of mocks.

The `dev` branch depends on currently private repos, but you can use published versions instead. Please refer to [`CHANGELOG`](./CHANGELOG.md) for features and update information.

## Table of Contents

- [Getting Started](#getting-started)
- [Debugging](#debugging)
- [Using Module Wrapper](#using-module-wrapper)
- [EVM Support](#evm-support)
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
            Coin::new(1_000_000_000_000u128, "usdt"),
            Coin::new(1_000_000_000_000u128, "inj"),
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
    Coin::new(1_000_000_000_000u128, "usdt"),
    Coin::new(1_000_000_000_000u128, "inj"),
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
            Coin::new(1_000_000_000_000u128, "usdt"),
            Coin::new(1_000_000_000_000u128, "inj"),
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
            Coin::new(1_000_000_000_000u128, "usdt"),
            Coin::new(1_000_000_000_000u128, "inj"),
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
            Coin::new(1_000_000_000_000u128, "usdt"),
            Coin::new(1_000_000_000_000u128, "inj"),
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

Additional examples can be found in the [modules](./src/module/) directory.

The wrapper surface now also includes first-class `Evm` and `Erc20` modules alongside the existing Cosmos and Wasm helpers.

## EVM Support

`injective-test-tube` now exposes the Injective EVM surface directly.

Available `Evm` features include:

- EVM account, balance, storage, code, and params queries
- `eth_call` and `estimate_gas`
- execution of raw signed Ethereum transactions through the canonical `ExtensionOptionsEthereumTx` path used by Injective Core
- Rust-side signing helpers for:
  - legacy Ethereum transactions
  - access-list Ethereum transactions
  - dynamic-fee Ethereum transactions

The crate also exposes an `Erc20` module for basic ERC20 token-pair queries.

The design intentionally keeps EVM execution aligned with core behavior: Rust signing helpers produce raw Ethereum transaction bytes, and the test environment submits them through the real Injective EVM transaction envelope instead of a mock or custom shortcut path.

## Versioning

The version of injective-test-tube is determined by the version of injective-core it follows. Changes made to test-tube or injective-test-tube will be notified by a new **release** candidate marker e.g. `1.14.0-rc1`.
