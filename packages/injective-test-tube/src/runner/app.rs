use cosmrs::Any;

use cosmwasm_std::Coin;

use prost::Message;
use test_tube_inj::account::SigningAccount;

use test_tube_inj::runner::result::{RunnerExecuteResult, RunnerExecuteResultMult, RunnerResult};
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
    /// Convinience function to create multiple accounts with the same
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

    /// Set parameter set for a given subspace.
    pub fn set_param_set(&self, subspace: &str, pset: impl Into<Any>) -> RunnerResult<()> {
        self.inner.set_param_set(subspace, pset)
    }

    /// Get parameter set for a given subspace.
    pub fn get_param_set<P: Message + Default>(
        &self,
        subspace: &str,
        type_url: &str,
    ) -> RunnerResult<P> {
        self.inner.get_param_set(subspace, type_url)
    }

    pub fn enable_increasing_block_time_in_end_blocker(&self) {
        self.inner.enable_increasing_block_time_in_end_blocker()
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

    fn execute_single_block<M, R>(
        &self,
        msgs: &[(M, &str, &SigningAccount)],
    ) -> RunnerExecuteResultMult<R>
    where
        M: ::prost::Message,
        R: ::prost::Message + Default,
    {
        self.inner.execute_single_block(msgs)
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
    use cosmwasm_std::{coins, Coin};
    use injective_std::types::{
        cosmos::bank::v1beta1::QueryAllBalancesRequest,
        injective::tokenfactory::v1beta1::{
            MsgCreateDenom, MsgCreateDenomResponse, QueryDenomsFromCreatorRequest,
            QueryParamsRequest, QueryParamsResponse,
        },
    };

    use crate::module::{TokenFactory, Wasm};
    use crate::runner::app::InjectiveTestApp;
    use crate::Bank;
    use test_tube_inj::account::{Account, FeeSetting};
    use test_tube_inj::module::Module;
    use test_tube_inj::runner::*;
    use test_tube_inj::ExecuteResponse;

    #[test]
    fn test_init_accounts() {
        let app = InjectiveTestApp::default();
        let accounts = app
            .init_accounts(&coins(100_000_000_000, "inj"), 3)
            .unwrap();

        assert!(accounts.get(0).is_some());
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

        assert_eq!(app.get_block_height(), 1i64);

        app.increase_time(10u64);

        assert_eq!(app.get_block_height(), 2i64);
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
            name: "denom".to_owned(),
            symbol: "DNM".to_owned(),
        };

        let res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute(msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom", &acc)
            .unwrap();

        let create_denom_attrs = &res.data.new_token_denom;
        assert_eq!(
            create_denom_attrs,
            &format!("factory/{}/{}", &addr, "newdenom")
        );

        // execute on more time to excercise account sequence
        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "newerdenom".to_string(),
            name: "newer denom".to_owned(),
            symbol: "NDNM".to_owned(),
        };

        let res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute(msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom", &acc)
            .unwrap();

        let create_denom_attrs = &res.data.new_token_denom;
        assert_eq!(
            create_denom_attrs,
            &format!("factory/{}/{}", &addr, "newerdenom")
        );

        // execute on more time to excercise account sequence
        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_1".to_string(),
            name: "multi denom 1".to_owned(),
            symbol: "MDNM".to_owned(),
        };

        let msg_2 = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_2".to_string(),
            name: "multi denom 2".to_owned(),
            symbol: "MDNM2".to_owned(),
        };

        assert_eq!(app.get_block_height(), 4i64);

        let _res: ExecuteResponse<MsgCreateDenomResponse> = app
            .execute_multiple(
                &[
                    (msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom"),
                    (msg_2, "/injective.tokenfactory.v1beta1.MsgCreateDenom"),
                ],
                &acc,
            )
            .unwrap();

        assert_eq!(app.get_block_height(), 5i64);

        let acc_2 = app
            .init_account(&coins(100_000_000_000_000_000_000u128, "inj")) // 100 inj
            .unwrap();
        assert_eq!(app.get_block_height(), 6i64);

        // execute on more time to excercise account sequence
        let msg = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_3".to_string(),
            name: "multi denom 3".to_owned(),
            symbol: "MDNM3".to_owned(),
        };

        let msg_2 = MsgCreateDenom {
            sender: acc.address(),
            subdenom: "multidenom_4".to_string(),
            name: "multi denom 4".to_owned(),
            symbol: "MDNM4".to_owned(),
        };

        let msg_3 = MsgCreateDenom {
            sender: acc_2.address(),
            subdenom: "multidenom_5".to_string(),
            name: "multi denom 5".to_owned(),
            symbol: "MDMA".to_owned(),
        };

        let res: Vec<ExecuteResponse<MsgCreateDenomResponse>> = app
            .execute_single_block(&[
                (msg, "/injective.tokenfactory.v1beta1.MsgCreateDenom", &acc),
                (
                    msg_2,
                    "/injective.tokenfactory.v1beta1.MsgCreateDenom",
                    &acc,
                ),
                (
                    msg_3,
                    "/injective.tokenfactory.v1beta1.MsgCreateDenom",
                    &acc_2,
                ),
            ])
            .unwrap();

        assert_eq!(res.len(), 3);

        assert_eq!(app.get_block_height(), 7i64);

        let tokenfactory = TokenFactory::new(&app);

        // Ensure denoms are created by acc
        let denoms = tokenfactory
            .query_denoms_from_creator(&QueryDenomsFromCreatorRequest {
                creator: acc.address(),
            })
            .unwrap()
            .denoms;

        assert_eq!(denoms.len(), 6);

        // Ensure denoms are created by acc_2
        let denoms = tokenfactory
            .query_denoms_from_creator(&QueryDenomsFromCreatorRequest {
                creator: acc_2.address(),
            })
            .unwrap()
            .denoms;

        assert_eq!(denoms.len(), 1);
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

        assert_eq!(
            denom_creation_fee,
            [Coin::new(10_000_000_000_000_000_000u128, "inj").into()]
        )
    }

    #[test]
    fn test_wasm_migrate() {
        use cosmwasm_std::Empty;
        use cw1_whitelist::msg::*;

        let app = InjectiveTestApp::default();
        let accs = app
            .init_accounts(
                &[
                    Coin::new(1_000_000_000_000, "uatom"),
                    Coin::new(1_000_000_000_000, "inj"),
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
                    Coin::new(1_000_000_000_000, "uatom"),
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
    fn test_custom_fee() {
        let app = InjectiveTestApp::default();
        let initial_balance = 1_000_000_000_000;
        let alice = app.init_account(&coins(initial_balance, "inj")).unwrap();
        let bob = app.init_account(&coins(initial_balance, "inj")).unwrap();

        let amount = Coin::new(1_000_000, "inj");
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
        assert_eq!(bob_balance, initial_balance - amount.amount.u128());
    }
}
