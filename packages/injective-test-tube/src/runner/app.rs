use cosmrs::proto::tendermint::v0_38::abci::ResponseFinalizeBlock;
use cosmwasm_std::Coin;
use prost::Message;
use test_tube_inj::account::{AddressDerivation, SigningAccount};
use test_tube_inj::runner::result::{RunnerExecuteResult, RunnerResult};
use test_tube_inj::runner::Runner;
use test_tube_inj::BaseApp;

const FEE_DENOM: &str = "inj";
const INJ_ADDRESS_PREFIX: &str = "inj";
const CHAIN_ID: &str = "injective-777";
const DEFAULT_GAS_ADJUSTMENT: f64 = 1.2;

#[derive(Debug, PartialEq)]
pub struct InjectiveTestApp {
    inner: BaseApp,
}

impl Default for InjectiveTestApp {
    fn default() -> Self {
        InjectiveTestApp::new()
    }
}

impl InjectiveTestApp {
    pub fn new() -> Self {
        Self {
            inner: BaseApp::new(
                FEE_DENOM,
                CHAIN_ID,
                INJ_ADDRESS_PREFIX,
                DEFAULT_GAS_ADJUSTMENT,
            ),
        }
    }

    /// Get the current block time in nanoseconds
    pub fn get_block_time_nanos(&self) -> i64 {
        self.inner.get_block_time_nanos()
    }

    /// Get the current block time in seconds
    pub fn get_block_time_seconds(&self) -> i64 {
        self.inner.get_block_time_nanos() / 1_000_000_000i64
    }

    /// Get the current block height
    pub fn get_block_height(&self) -> i64 {
        self.inner.get_block_height()
    }

    pub fn get_account_sequence(&self, address: &str) -> u64 {
        self.inner.get_account_sequence(address)
    }

    pub fn get_account_number(&self, address: &str) -> u64 {
        self.inner.get_account_number(address)
    }

    /// Get the first validator address
    pub fn get_first_validator_address(&self) -> RunnerResult<String> {
        self.inner.get_first_validator_address()
    }

    /// Get the first validator private key
    pub fn get_first_validator_private_key(&self) -> RunnerResult<String> {
        self.inner.get_first_validator_private_key()
    }

    /// Get the first validator signing account
    pub fn get_first_validator_signing_account(
        &self,
        denom: String,
        gas_adjustment: f64,
    ) -> RunnerResult<SigningAccount> {
        self.inner
            .get_first_validator_signing_account(denom, gas_adjustment)
    }

    /// Increase the time of the blockchain by the given number of seconds.
    pub fn increase_time(&self, seconds: u64) {
        self.inner.increase_time(seconds)
    }

    /// Initialize account with initial balance of any coins.
    /// This function mints new coins and send to newly created account
    pub fn init_account(&self, coins: &[Coin]) -> RunnerResult<SigningAccount> {
        self.inner.init_account(coins)
    }

    /// Initialize account with initial balance of any coins using an explicit
    /// address derivation mode.
    pub fn init_account_with_derivation(
        &self,
        coins: &[Coin],
        derivation: AddressDerivation,
    ) -> RunnerResult<SigningAccount> {
        self.inner.init_account_with_derivation(coins, derivation)
    }

    /// Initialize account with initial balance of any coins, defining decimals if not created.
    /// This function mints new coins and send to newly created account
    pub fn init_account_decimals(
        &self,
        coins: &[Coin],
        decimals: &[u32],
    ) -> RunnerResult<SigningAccount> {
        self.inner.init_account_decimals(coins, decimals)
    }

    /// Initialize account with initial balance of any coins, defining decimals
    /// if not created, using an explicit address derivation mode.
    pub fn init_account_decimals_with_derivation(
        &self,
        coins: &[Coin],
        decimals: &[u32],
        derivation: AddressDerivation,
    ) -> RunnerResult<SigningAccount> {
        self.inner
            .init_account_decimals_with_derivation(coins, decimals, derivation)
    }

    /// Convenience function to create multiple accounts with the same
    /// Initial coins balance
    pub fn init_accounts(&self, coins: &[Coin], count: u64) -> RunnerResult<Vec<SigningAccount>> {
        self.inner.init_accounts(coins, count)
    }

    /// Simulate transaction execution and return gas info
    pub fn simulate_tx<I>(
        &self,
        msgs: I,
        signer: &SigningAccount,
    ) -> RunnerResult<cosmrs::proto::cosmos::base::abci::v1beta1::GasInfo>
    where
        I: IntoIterator<Item = cosmrs::Any>,
    {
        self.inner.simulate_tx(msgs, signer)
    }

    /// Get parameter set for a given subspace.
    pub fn get_param_set<P: Message + Default>(
        &self,
        subspace: &str,
        type_url: &str,
    ) -> RunnerResult<P> {
        self.inner.get_param_set(subspace, type_url)
    }

    pub fn execute_signed_evm_txs_raw_response(
        &self,
        raw_txs: &[Vec<u8>],
    ) -> RunnerResult<ResponseFinalizeBlock> {
        self.inner.execute_signed_evm_txs_raw_response(raw_txs)
    }
}

impl<'a> Runner<'a> for InjectiveTestApp {
    fn execute_multiple<M, R>(
        &self,
        msgs: &[(M, &str)],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<R>
    where
        M: ::prost::Message,
        R: ::prost::Message + Default,
    {
        self.inner.execute_multiple(msgs, signer)
    }

    fn query<Q, R>(&self, path: &str, q: &Q) -> RunnerResult<R>
    where
        Q: ::prost::Message,
        R: ::prost::Message + Default,
    {
        self.inner.query(path, q)
    }

    fn execute_multiple_raw<R>(
        &self,
        msgs: Vec<cosmrs::Any>,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<R>
    where
        R: prost::Message + Default,
    {
        self.inner.execute_multiple_raw(msgs, signer)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Coin, Uint256};
    use injective_std::types::{
        cosmos::bank::v1beta1::{
            QueryAllBalancesRequest, QueryBalanceRequest, QueryDenomMetadataRequest,
        },
        injective::tokenfactory::v1beta1::{
            MsgCreateDenom, MsgCreateDenomResponse, QueryParamsRequest, QueryParamsResponse,
        },
    };

    use crate::module::Wasm;
    use crate::runner::app::InjectiveTestApp;
    use crate::{derive_evm_address, derive_injective_evm_address, Authz, Bank, Evm, EvmLegacyTx};
    use test_tube_inj::account::{Account, AddressDerivation, FeeSetting};
    use test_tube_inj::module::Module;
    use test_tube_inj::runner::*;
    use test_tube_inj::ExecuteResponse;

    #[test]
    fn test_init_accounts() {
        let app = InjectiveTestApp::default();
        let accounts = app
            .init_accounts(&coins(100_000_000_000, "inj"), 3)
            .unwrap();

        assert!(!accounts.is_empty());
        assert!(accounts.get(1).is_some());
        assert!(accounts.get(2).is_some());
        assert!(accounts.get(3).is_none());
    }

    #[test]
    fn test_get_and_set_block_timestamp() {
        let app = InjectiveTestApp::default();

        let block_time_nanos = app.get_block_time_nanos();
        let block_time_seconds = app.get_block_time_seconds();

        app.increase_time(10u64);

        assert_eq!(
            app.get_block_time_nanos(),
            block_time_nanos + 10_000_000_000
        );
        assert_eq!(app.get_block_time_seconds(), block_time_seconds + 10);
    }

    #[test]
    fn test_get_block_height() {
        let app = InjectiveTestApp::default();

        assert_eq!(app.get_block_height(), 500i64);

        app.increase_time(10u64);

        assert_eq!(app.get_block_height(), 501i64);
    }

    #[test]
    fn test_execute() {
        let app = InjectiveTestApp::default();

        let acc = app
            .init_account(&coins(100_000_000_000_000_000_000u128, "inj")) // 100 inj
            .unwrap();
        let addr = acc.address();

        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "newdenom".to_string(),
            name: "token_name".to_owned(),
            symbol: "SYM".to_owned(),
            decimals: 6,
            allow_admin_burn: true,
        };

        let res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute(msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom", &acc)
            .unwrap();

        let create_denom_attrs = &res.data.new_token_denom;
        assert_eq!(
            create_denom_attrs,
            &format!("factory/{}/{}", &addr, "newdenom")
        );

        // execute on more time to exercise account sequence
        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "newerdenom".to_string(),
            name: "token_name".to_owned(),
            symbol: "SYM".to_owned(),
            decimals: 6,
            allow_admin_burn: true,
        };

        let res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute(msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom", &acc)
            .unwrap();

        let create_denom_attrs = &res.data.new_token_denom;
        assert_eq!(
            create_denom_attrs,
            &format!("factory/{}/{}", &addr, "newerdenom")
        );

        // execute on more time to exercise account sequence
        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_1".to_string(),
            name: "token_name".to_owned(),
            symbol: "SYM".to_owned(),
            decimals: 6,
            allow_admin_burn: true,
        };

        let msg_2 = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_2".to_string(),
            name: "token_name".to_owned(),
            symbol: "SYM".to_owned(),
            decimals: 6,
            allow_admin_burn: true,
        };

        let mut current_block_height = 503i64;
        assert_eq!(app.get_block_height(), current_block_height);

        let _res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute_multiple(
                &[
                    (msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom"),
                    (msg_2, "/injective.tokenfactory.v1beta1.MsgCreateDenom"),
                ],
                &acc,
            )
            .unwrap();
        current_block_height += 1;
        assert_eq!(app.get_block_height(), current_block_height);

        app.init_account(&coins(100_000_000_000_000_000_000u128, "inj")) // 100 inj
            .unwrap();
        current_block_height += 1;
        assert_eq!(app.get_block_height(), current_block_height);
    }

    #[test]
    fn test_query() {
        let app = InjectiveTestApp::default();

        let denom_creation_fee = app
            .query::<QueryParamsRequest, QueryParamsResponse>(
                "/injective.tokenfactory.v1beta1.Query/Params",
                &QueryParamsRequest {},
            )
            .unwrap()
            .params
            .unwrap()
            .denom_creation_fee;

        assert_eq!(denom_creation_fee.len(), 1);
        assert_eq!(
            denom_creation_fee.first().unwrap().amount,
            "10000000000000000000".to_string()
        );
        assert_eq!(denom_creation_fee.first().unwrap().denom, "inj".to_string());
    }

    #[test]
    fn test_wasm_migrate() {
        use cosmwasm_std::Empty;
        use cw1_whitelist::msg::*;

        let app = InjectiveTestApp::default();
        let accs = app
            .init_accounts(
                &[
                    Coin::new(1_000_000_000_000u128, "uatom"),
                    Coin::new(1_000_000_000_000u128, "inj"),
                ],
                1,
            )
            .unwrap();
        let admin = &accs[0];

        let wasm = Wasm::new(&app);
        let wasm_byte_code = std::fs::read("./test_artifacts/cw1_subkeys.wasm").unwrap();
        let code_id = wasm
            .store_code(&wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;
        assert_eq!(code_id, 1);

        // initialize admins and check if the state is correct
        let init_admins = vec![admin.address()];
        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    admins: init_admins.clone(),
                    mutable: true,
                },
                Some(&admin.address()),
                Some("Test label"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;
        let admin_list = wasm
            .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
            .unwrap();
        assert_eq!(admin_list.admins, init_admins);
        assert!(admin_list.mutable);

        let code_id = wasm
            .store_code(&wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;
        assert_eq!(code_id, 2);

        wasm.migrate(code_id, &contract_addr, &Empty {}, admin)
            .unwrap();

        let admin_list = wasm
            .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
            .unwrap();
        assert_eq!(admin_list.admins, init_admins);
        assert!(admin_list.mutable);
    }

    #[test]
    fn test_wasm_execute_and_query() {
        use cw1_whitelist::msg::*;

        let app = InjectiveTestApp::default();
        let accs = app
            .init_accounts(
                &[
                    Coin::new(1_000_000_000_000u128, "uatom"),
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
        assert_eq!(code_id, 1);

        // initialize admins and check if the state is correct
        let init_admins = vec![admin.address()];
        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    admins: init_admins.clone(),
                    mutable: true,
                },
                Some(&admin.address()),
                Some("Test label"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;
        let admin_list = wasm
            .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
            .unwrap();
        assert_eq!(admin_list.admins, init_admins);
        assert!(admin_list.mutable);

        // update admin and check again
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
    }

    #[test]
    fn test_injective_evm_identity_bank_authz_wasm_and_evm() {
        use cw1_whitelist::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
        use injective_std::shim::Any;
        use injective_std::types::{
            cosmos::{
                authz::v1beta1::{
                    GenericAuthorization, Grant, MsgGrant, QueryGranteeGrantsRequest,
                },
                bank::v1beta1::{MsgSend, QueryBalanceRequest},
                base::v1beta1::Coin as BaseCoin,
            },
            injective::evm::v1::QueryAccountRequest,
        };
        use prost::Message as _;

        let app = InjectiveTestApp::default();
        let bank = Bank::new(&app);
        let authz = Authz::new(&app);
        let wasm = Wasm::new(&app);
        let evm = Evm::new(&app);

        let signer = app
            .init_account_with_derivation(
                &[
                    Coin::new(1_000_000_000_000u128, "inj"),
                    Coin::new(10u128, "usdc"),
                ],
                AddressDerivation::InjectiveEvm,
            )
            .unwrap();
        let receiver = app.init_account(&[Coin::new(1u128, "inj")]).unwrap();
        let new_admin = app.init_account(&[Coin::new(1u128, "inj")]).unwrap();

        assert_eq!(signer.derivation(), AddressDerivation::InjectiveEvm);
        assert_eq!(signer.address(), derive_injective_evm_address(&signer));
        assert!(app.get_account_number(&signer.address()) > 0);
        assert_eq!(app.get_account_sequence(&signer.address()), 0);

        let bank_send_msg = MsgSend {
            from_address: signer.address(),
            to_address: receiver.address(),
            amount: vec![BaseCoin {
                amount: 9u128.to_string(),
                denom: "inj".to_string(),
            }],
        };

        let gas_info = app
            .simulate_tx(
                [cosmrs::Any {
                    type_url: "/cosmos.bank.v1beta1.MsgSend".to_string(),
                    value: bank_send_msg.encode_to_vec(),
                }],
                &signer,
            )
            .unwrap();
        assert!(gas_info.gas_used > 0);

        bank.send(bank_send_msg, &signer).unwrap();
        assert_eq!(app.get_account_sequence(&signer.address()), 1);

        let receiver_balance = bank
            .query_balance(&QueryBalanceRequest {
                address: receiver.address(),
                denom: "inj".to_string(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount
            .parse::<u128>()
            .unwrap();
        assert_eq!(receiver_balance, 10);

        let generic_auth = GenericAuthorization {
            msg: "/cosmos.bank.v1beta1.MsgSend".to_string(),
        };
        let mut authorization_bytes = Vec::new();
        generic_auth.encode(&mut authorization_bytes).unwrap();

        authz
            .grant(
                MsgGrant {
                    granter: signer.address(),
                    grantee: receiver.address(),
                    grant: Some(Grant {
                        authorization: Some(Any {
                            type_url: "/cosmos.authz.v1beta1.GenericAuthorization".to_string(),
                            value: authorization_bytes.clone(),
                        }),
                        expiration: None,
                    }),
                },
                &signer,
            )
            .unwrap();

        let grants = authz
            .query_grantee_grants(&QueryGranteeGrantsRequest {
                grantee: receiver.address(),
                pagination: None,
            })
            .unwrap();
        assert_eq!(grants.grants.len(), 1);
        assert_eq!(grants.grants[0].granter, signer.address());
        assert_eq!(grants.grants[0].grantee, receiver.address());

        let wasm_byte_code = std::fs::read("./test_artifacts/cw1_whitelist.wasm").unwrap();
        let code_id = wasm
            .store_code(&wasm_byte_code, None, &signer)
            .unwrap()
            .data
            .code_id;
        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    admins: vec![signer.address()],
                    mutable: true,
                },
                Some(&signer.address()),
                Some("InjectiveEvm identity"),
                &[],
                &signer,
            )
            .unwrap()
            .data
            .address;
        let admin_list = wasm
            .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
            .unwrap();
        assert_eq!(admin_list.admins, vec![signer.address()]);

        wasm.execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UpdateAdmins {
                admins: vec![new_admin.address()],
            },
            &[],
            &signer,
        )
        .unwrap();

        let updated_admins = wasm
            .query::<QueryMsg, AdminListResponse>(&contract_addr, &QueryMsg::AdminList {})
            .unwrap();
        assert_eq!(updated_admins.admins, vec![new_admin.address()]);

        let signer_evm_address = derive_evm_address(&signer);
        let signer_evm_account = evm
            .query_account(&QueryAccountRequest {
                address: signer_evm_address.clone(),
            })
            .unwrap();
        assert_eq!(
            signer_evm_account.nonce,
            app.get_account_sequence(&signer.address())
        );
        assert!(!signer_evm_account.balance.is_empty());

        let evm_recipient = app.init_account(&[Coin::new(1u128, "inj")]).unwrap();
        let evm_transfer = evm
            .execute_legacy_tx(
                &signer,
                &EvmLegacyTx {
                    nonce: signer_evm_account.nonce,
                    gas_price: 2_500u128,
                    gas_limit: 21_000,
                    to: Some(derive_evm_address(&evm_recipient)),
                    value: 123u128,
                    data: Vec::new(),
                    chain_id: None,
                },
            )
            .unwrap();
        assert!(evm_transfer.data.vm_error.is_empty());

        let signer_evm_account_after = evm
            .query_account(&QueryAccountRequest {
                address: signer_evm_address,
            })
            .unwrap();
        assert_eq!(signer_evm_account_after.nonce, signer_evm_account.nonce + 1);
    }

    #[test]
    fn test_init_account_decimals_with_injective_evm_derivation() {
        let app = InjectiveTestApp::default();
        let bank = Bank::new(&app);

        let signer = app
            .init_account_decimals_with_derivation(
                &[
                    Coin::new(123_000_000u128, "usdc"),
                    Coin::new(456_000_000_000_000_000u128, "inj"),
                ],
                &[6u32, 18u32],
                AddressDerivation::InjectiveEvm,
            )
            .unwrap();

        assert_eq!(signer.derivation(), AddressDerivation::InjectiveEvm);
        assert_eq!(signer.address(), derive_injective_evm_address(&signer));
        assert!(app.get_account_number(&signer.address()) > 0);

        let usdc_balance = bank
            .query_balance(&QueryBalanceRequest {
                address: signer.address(),
                denom: "usdc".to_string(),
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;
        assert_eq!(usdc_balance, "123000000");

        let usdc_decimals = bank
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: "usdc".to_string(),
            })
            .unwrap()
            .metadata
            .unwrap()
            .decimals;
        assert_eq!(usdc_decimals, 6);
    }

    #[test]
    fn test_custom_fee() {
        let app = InjectiveTestApp::default();
        let initial_balance = 1_000_000_000_000u128;
        let alice = app.init_account(&coins(initial_balance, "inj")).unwrap();
        let bob = app.init_account(&coins(initial_balance, "inj")).unwrap();

        let amount = Coin::new(1_000_000u128, "inj");
        let gas_limit = 100_000_000;

        // use FeeSetting::Auto by default, so should not equal newly custom fee setting
        let wasm = Wasm::new(&app);
        let wasm_byte_code = std::fs::read("./test_artifacts/cw1_whitelist.wasm").unwrap();
        let res = wasm.store_code(&wasm_byte_code, None, &alice).unwrap();

        assert_ne!(res.gas_info.gas_wanted, gas_limit);

        //update fee setting
        let bob = bob.with_fee_setting(FeeSetting::Custom {
            amount: amount.clone(),
            gas_limit,
        });
        let res = wasm.store_code(&wasm_byte_code, None, &bob).unwrap();

        let bob_balance = Bank::new(&app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: bob.address(),
                pagination: None,
                resolve_denom: false,
            })
            .unwrap()
            .balances
            .into_iter()
            .find(|c| c.denom == "inj")
            .unwrap()
            .amount
            .parse::<u128>()
            .unwrap();

        assert_eq!(res.gas_info.gas_wanted, gas_limit);
        assert_eq!(
            Uint256::new(bob_balance),
            Uint256::new(initial_balance) - amount.amount
        );
    }
}
