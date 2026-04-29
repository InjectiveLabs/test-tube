use cosmrs::proto::{
    cosmos::base::abci::v1beta1::{GasInfo, TxMsgData},
    tendermint::v0_38::abci::{ExecTxResult, ResponseFinalizeBlock},
};
use cosmwasm_std::{Attribute, Event};
use injective_std::types::injective::evm::v1::{
    EstimateGasResponse, EthCallRequest, MsgEthereumTxResponse, QueryAccountRequest,
    QueryAccountResponse, QueryBalanceRequest, QueryBalanceResponse, QueryCodeRequest,
    QueryCodeResponse, QueryParamsRequest, QueryParamsResponse, QueryStorageRequest,
    QueryStorageResponse,
};
use prost::Message;
use serde_json::{Map, Value};
use test_tube_inj::{
    fn_query,
    module::Module,
    runner::{
        error::{DecodeError, EncodeError, RunnerError},
        result::{ExecuteResponse, RunnerResult},
        Runner,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvmCall {
    pub from: Option<String>,
    pub to: Option<String>,
    pub gas: Option<u64>,
    pub value: Option<u128>,
    pub nonce: Option<u64>,
    pub data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvmQueryOptions {
    pub gas_cap: u64,
    pub proposer_address: Option<Vec<u8>>,
    pub chain_id: i64,
    pub overrides: Option<Value>,
}

pub struct Evm<'a, R: Runner<'a>> {
    runner: &'a R,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvmExecuteResponse {
    pub responses: Vec<MsgEthereumTxResponse>,
    pub raw_data: Vec<u8>,
    pub events: Vec<Event>,
    pub gas_info: GasInfo,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Evm<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Evm<'a, R>
where
    R: Runner<'a>,
{
    fn build_eth_call_request(
        &self,
        call: &EvmCall,
        options: &EvmQueryOptions,
    ) -> RunnerResult<EthCallRequest> {
        Ok(EthCallRequest {
            args: serde_json::to_vec(&build_call_args(call))
                .map_err(EncodeError::JsonEncodeError)
                .map_err(RunnerError::EncodeError)?,
            gas_cap: options.gas_cap,
            proposer_address: options.proposer_address.clone().unwrap_or_default(),
            chain_id: options.chain_id,
            overrides: match &options.overrides {
                Some(overrides) => serde_json::to_vec(overrides)
                    .map_err(EncodeError::JsonEncodeError)
                    .map_err(RunnerError::EncodeError)?,
                None => Vec::new(),
            },
        })
    }

    fn_query! {
        pub query_account ["/injective.evm.v1.Query/Account"]: QueryAccountRequest => QueryAccountResponse
    }

    fn_query! {
        pub query_balance ["/injective.evm.v1.Query/Balance"]: QueryBalanceRequest => QueryBalanceResponse
    }

    fn_query! {
        pub query_storage ["/injective.evm.v1.Query/Storage"]: QueryStorageRequest => QueryStorageResponse
    }

    fn_query! {
        pub query_code ["/injective.evm.v1.Query/Code"]: QueryCodeRequest => QueryCodeResponse
    }

    fn_query! {
        pub query_params ["/injective.evm.v1.Query/Params"]: QueryParamsRequest => QueryParamsResponse
    }

    pub fn query_eth_call(&self, msg: &EthCallRequest) -> RunnerResult<MsgEthereumTxResponse> {
        self.runner
            .query::<EthCallRequest, MsgEthereumTxResponse>("/injective.evm.v1.Query/EthCall", msg)
    }

    pub fn query_estimate_gas(&self, msg: &EthCallRequest) -> RunnerResult<EstimateGasResponse> {
        self.runner.query::<EthCallRequest, EstimateGasResponse>(
            "/injective.evm.v1.Query/EstimateGas",
            msg,
        )
    }

    pub fn eth_call(
        &self,
        call: &EvmCall,
        options: &EvmQueryOptions,
    ) -> RunnerResult<MsgEthereumTxResponse> {
        self.query_eth_call(&self.build_eth_call_request(call, options)?)
    }

    pub fn estimate_gas(
        &self,
        call: &EvmCall,
        options: &EvmQueryOptions,
    ) -> RunnerResult<EstimateGasResponse> {
        self.query_estimate_gas(&self.build_eth_call_request(call, options)?)
    }
}

fn build_call_args(call: &EvmCall) -> Value {
    let mut args = Map::new();

    if let Some(from) = &call.from {
        args.insert("from".to_string(), Value::String(from.clone()));
    }

    if let Some(to) = &call.to {
        args.insert("to".to_string(), Value::String(to.clone()));
    }

    if let Some(gas) = call.gas {
        args.insert("gas".to_string(), Value::String(format_quantity(gas)));
    }

    if let Some(value) = call.value {
        args.insert("value".to_string(), Value::String(format_quantity(value)));
    }

    if let Some(nonce) = call.nonce {
        args.insert("nonce".to_string(), Value::String(format_quantity(nonce)));
    }

    if let Some(data) = &call.data {
        args.insert("input".to_string(), Value::String(format_bytes(data)));
    }

    Value::Object(args)
}

fn format_quantity<T>(value: T) -> String
where
    T: std::fmt::LowerHex,
{
    format!("0x{value:x}")
}

fn format_bytes(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(2 + bytes.len() * 2);
    encoded.push_str("0x");

    for byte in bytes {
        encoded.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        encoded.push(HEX_DIGITS[(byte & 0x0f) as usize] as char);
    }

    encoded
}

fn decode_events(tx_result: &ExecTxResult) -> Result<Vec<Event>, DecodeError> {
    tx_result
        .events
        .iter()
        .cloned()
        .map(|event| -> Result<Event, DecodeError> {
            Ok(Event::new(event.r#type).add_attributes(
                event
                    .attributes
                    .into_iter()
                    .map(|attribute| Attribute {
                        key: attribute.key,
                        value: attribute.value,
                    })
                    .collect::<Vec<_>>(),
            ))
        })
        .collect()
}

fn decode_evm_execute_response(res: ResponseFinalizeBlock) -> RunnerResult<EvmExecuteResponse> {
    let tx = res.tx_results.first().ok_or(RunnerError::ExecuteError {
        msg: "No tx results".to_string(),
    })?;

    let tx_msg_data = TxMsgData::decode(tx.data.as_ref()).map_err(DecodeError::ProtoDecodeError)?;
    let responses = tx_msg_data
        .msg_responses
        .iter()
        .map(|msg_data| {
            MsgEthereumTxResponse::decode(msg_data.value.as_slice())
                .map_err(DecodeError::ProtoDecodeError)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(RunnerError::DecodeError)?;

    if responses.is_empty() {
        return Err(RunnerError::ExecuteError {
            msg: tx.log.clone(),
        });
    }

    Ok(EvmExecuteResponse {
        responses,
        raw_data: tx.data.to_vec(),
        events: decode_events(tx).map_err(RunnerError::DecodeError)?,
        gas_info: GasInfo {
            gas_wanted: tx.gas_wanted as u64,
            gas_used: tx.gas_used as u64,
        },
    })
}

impl<'a> Evm<'a, crate::InjectiveTestApp> {
    pub fn execute_raw_ethereum_txs(
        &self,
        raw_txs: &[Vec<u8>],
    ) -> RunnerResult<EvmExecuteResponse> {
        let res = self.runner.execute_signed_evm_txs_raw_response(raw_txs)?;
        decode_evm_execute_response(res)
    }

    pub fn execute_raw_ethereum_tx(
        &self,
        raw_tx: Vec<u8>,
    ) -> RunnerResult<ExecuteResponse<MsgEthereumTxResponse>> {
        let response = self.execute_raw_ethereum_txs(&[raw_tx])?;
        let data = response
            .responses
            .into_iter()
            .next()
            .ok_or(RunnerError::ExecuteError {
                msg: "No EVM message responses".to_string(),
            })?;

        Ok(ExecuteResponse {
            data,
            raw_data: response.raw_data,
            events: response.events,
            gas_info: response.gas_info,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine as _;
    use cosmrs::{AccountId, Any};
    use injective_std::types::injective::evm::v1::{
        QueryAccountRequest, QueryBalanceRequest, QueryCodeRequest, QueryParamsRequest,
        QueryStorageRequest,
    };
    use k256::{
        ecdsa::SigningKey as K256SigningKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey,
    };
    use prost::Message;
    use rlp::RlpStream;
    use serde_json::{json, Value};
    use sha3::{Digest, Keccak256};
    use test_tube_inj::{
        account::{Account, SigningAccount},
        runner::{result::RunnerExecuteResult, Runner},
        Module,
    };

    use crate::{
        injective_std::types::cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin as BaseCoin},
        Bank, Evm, EvmCall, EvmQueryOptions, InjectiveTestApp, RunnerResult,
    };

    struct DerivedEvmAccount {
        inj_address: String,
        eth_address: String,
    }

    #[derive(Default)]
    struct FakeRunner {
        last_path: RefCell<Option<String>>,
        last_query: RefCell<Vec<u8>>,
    }

    impl FakeRunner {
        fn decode_last_query<Q>(&self) -> Q
        where
            Q: Message + Default,
        {
            Q::decode(self.last_query.borrow().as_slice()).expect("captured query should decode")
        }

        fn last_path(&self) -> Option<String> {
            self.last_path.borrow().clone()
        }
    }

    impl<'a> Runner<'a> for FakeRunner {
        fn execute_multiple<M, R>(
            &self,
            _msgs: &[(M, &str)],
            _signer: &SigningAccount,
        ) -> RunnerExecuteResult<R>
        where
            M: Message,
            R: Message + Default,
        {
            unreachable!("FakeRunner is only used for EVM query tests")
        }

        fn execute_multiple_raw<R>(
            &self,
            _msgs: Vec<Any>,
            _signer: &SigningAccount,
        ) -> RunnerExecuteResult<R>
        where
            R: Message + Default,
        {
            unreachable!("FakeRunner is only used for EVM query tests")
        }

        fn query<Q, R>(&self, path: &str, query: &Q) -> RunnerResult<R>
        where
            Q: Message,
            R: Message + Default,
        {
            let mut buf = Vec::new();
            query.encode(&mut buf).expect("query should encode");

            *self.last_path.borrow_mut() = Some(path.to_string());
            *self.last_query.borrow_mut() = buf;

            Ok(R::default())
        }
    }

    fn derive_evm_account(account: &SigningAccount) -> DerivedEvmAccount {
        let pubkey_bytes = account.public_key().to_bytes();
        let pubkey = PublicKey::from_sec1_bytes(&pubkey_bytes)
            .expect("signing account should use a valid secp256k1 public key");
        let uncompressed = pubkey.to_encoded_point(false);
        let hash = Keccak256::digest(&uncompressed.as_bytes()[1..]);
        let eth_address_bytes = &hash[12..];

        DerivedEvmAccount {
            inj_address: AccountId::new("inj", eth_address_bytes)
                .expect("derived EVM bytes should form a valid Injective address")
                .to_string(),
            eth_address: super::format_bytes(eth_address_bytes),
        }
    }

    fn evm_chain_id(evm: &Evm<'_, InjectiveTestApp>) -> u64 {
        evm.query_params(&QueryParamsRequest {})
            .unwrap()
            .params
            .expect("EVM params should be present")
            .chain_config
            .expect("EVM chain config should be present")
            .eip155_chain_id
            .parse()
            .expect("EVM chain id should parse as u64")
    }

    fn decode_hex_nibble(ch: u8) -> u8 {
        match ch {
            b'0'..=b'9' => ch - b'0',
            b'a'..=b'f' => ch - b'a' + 10,
            b'A'..=b'F' => ch - b'A' + 10,
            _ => panic!("invalid hex character"),
        }
    }

    fn decode_hex_bytes(hex: &str) -> Vec<u8> {
        let trimmed = hex.strip_prefix("0x").unwrap_or(hex);
        assert_eq!(trimmed.len() % 2, 0, "hex input must have an even length");

        trimmed
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| (decode_hex_nibble(pair[0]) << 4) | decode_hex_nibble(pair[1]))
            .collect()
    }

    fn parse_evm_address(address: &str) -> [u8; 20] {
        decode_hex_bytes(address)
            .try_into()
            .expect("EVM address should contain 20 bytes")
    }

    fn trim_left_zeroes(bytes: &[u8]) -> Vec<u8> {
        let first_non_zero = bytes
            .iter()
            .position(|byte| *byte != 0)
            .unwrap_or(bytes.len());
        bytes[first_non_zero..].to_vec()
    }

    fn build_legacy_signed_raw_tx(
        private_key_base64: &str,
        chain_id: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        to: [u8; 20],
        value: u64,
        data: &[u8],
    ) -> Vec<u8> {
        let private_key_bytes = BASE64_STANDARD
            .decode(private_key_base64)
            .expect("validator private key should be valid base64");
        let signing_key = K256SigningKey::from_slice(&private_key_bytes)
            .expect("validator private key should be valid secp256k1 bytes");

        let mut signing_payload = RlpStream::new_list(9);
        signing_payload.append(&nonce);
        signing_payload.append(&gas_price);
        signing_payload.append(&gas_limit);
        signing_payload.append(&to.to_vec());
        signing_payload.append(&value);
        signing_payload.append(&data.to_vec());
        signing_payload.append(&chain_id);
        signing_payload.append(&0u8);
        signing_payload.append(&0u8);

        let mut digest = Keccak256::new();
        digest.update(signing_payload.out());

        let (signature, recovery_id) = signing_key
            .sign_digest_recoverable(digest)
            .expect("legacy transaction should sign");
        let signature_bytes = signature.to_bytes();
        let v = chain_id * 2 + 35 + u64::from(recovery_id.to_byte());

        let mut signed_tx = RlpStream::new_list(9);
        signed_tx.append(&nonce);
        signed_tx.append(&gas_price);
        signed_tx.append(&gas_limit);
        signed_tx.append(&to.to_vec());
        signed_tx.append(&value);
        signed_tx.append(&data.to_vec());
        signed_tx.append(&v);
        signed_tx.append(&trim_left_zeroes(&signature_bytes[..32]));
        signed_tx.append(&trim_left_zeroes(&signature_bytes[32..]));

        signed_tx.out().to_vec()
    }

    fn fund_evm_account(
        bank: &Bank<'_, InjectiveTestApp>,
        signer: &SigningAccount,
        destination: &str,
        amount: u128,
    ) {
        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: destination.to_string(),
                amount: vec![BaseCoin {
                    denom: "inj".to_string(),
                    amount: amount.to_string(),
                }],
            },
            signer,
        )
        .unwrap();
    }

    #[test]
    fn query_eth_call_uses_expected_path() {
        let runner = FakeRunner::default();
        let evm = Evm::new(&runner);

        evm.query_eth_call(&injective_std::types::injective::evm::v1::EthCallRequest {
            args: br#"{"to":"0x0000000000000000000000000000000000000001"}"#.to_vec(),
            gas_cap: 21_000,
            proposer_address: vec![],
            chain_id: 888,
            overrides: vec![],
        })
        .unwrap();

        assert_eq!(
            runner.last_path().as_deref(),
            Some("/injective.evm.v1.Query/EthCall")
        );
    }

    #[test]
    fn query_estimate_gas_uses_expected_path() {
        let runner = FakeRunner::default();
        let evm = Evm::new(&runner);

        evm.query_estimate_gas(&injective_std::types::injective::evm::v1::EthCallRequest {
            args: br#"{"to":"0x0000000000000000000000000000000000000001"}"#.to_vec(),
            gas_cap: 21_000,
            proposer_address: vec![],
            chain_id: 888,
            overrides: vec![],
        })
        .unwrap();

        assert_eq!(
            runner.last_path().as_deref(),
            Some("/injective.evm.v1.Query/EstimateGas")
        );
    }

    #[test]
    fn eth_call_helper_builds_expected_request() {
        let runner = FakeRunner::default();
        let evm = Evm::new(&runner);

        evm.eth_call(
            &EvmCall {
                from: Some("0x0000000000000000000000000000000000000002".to_string()),
                to: Some("0x0000000000000000000000000000000000000003".to_string()),
                gas: Some(21_000),
                value: Some(15),
                nonce: Some(7),
                data: Some(vec![0xde, 0xad, 0xbe, 0xef]),
            },
            &EvmQueryOptions {
                gas_cap: 500_000,
                proposer_address: Some(vec![1, 2, 3]),
                chain_id: 888,
                overrides: Some(json!({
                    "0x0000000000000000000000000000000000000003": {
                        "balance": "0x1"
                    }
                })),
            },
        )
        .unwrap();

        let request: injective_std::types::injective::evm::v1::EthCallRequest =
            runner.decode_last_query();

        assert_eq!(request.gas_cap, 500_000);
        assert_eq!(request.proposer_address, vec![1, 2, 3]);
        assert_eq!(request.chain_id, 888);
        assert_eq!(
            serde_json::from_slice::<Value>(&request.overrides).unwrap(),
            json!({
                "0x0000000000000000000000000000000000000003": {
                    "balance": "0x1"
                }
            })
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&request.args).unwrap(),
            json!({
                "from": "0x0000000000000000000000000000000000000002",
                "to": "0x0000000000000000000000000000000000000003",
                "gas": "0x5208",
                "value": "0xf",
                "nonce": "0x7",
                "input": "0xdeadbeef"
            })
        );
    }

    #[test]
    fn estimate_gas_helper_builds_expected_request() {
        let runner = FakeRunner::default();
        let evm = Evm::new(&runner);

        evm.estimate_gas(
            &EvmCall {
                to: Some("0x0000000000000000000000000000000000000004".to_string()),
                data: Some(vec![0xca, 0xfe]),
                ..Default::default()
            },
            &EvmQueryOptions {
                gas_cap: 21_000,
                proposer_address: None,
                chain_id: 999,
                overrides: None,
            },
        )
        .unwrap();

        let request: injective_std::types::injective::evm::v1::EthCallRequest =
            runner.decode_last_query();

        assert_eq!(request.gas_cap, 21_000);
        assert!(request.proposer_address.is_empty());
        assert_eq!(request.chain_id, 999);
        assert!(request.overrides.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&request.args).unwrap(),
            json!({
                "to": "0x0000000000000000000000000000000000000004",
                "input": "0xcafe"
            })
        );
    }

    #[test]
    fn evm_query_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let signer = app
            .init_account(&[cosmwasm_std::Coin::new(1_000_000_000u128, "inj")])
            .unwrap();
        let evm_account = derive_evm_account(&signer);

        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: evm_account.inj_address.clone(),
                amount: vec![BaseCoin {
                    denom: "inj".to_string(),
                    amount: "12345".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();

        let params = evm.query_params(&QueryParamsRequest {}).unwrap();
        assert!(params.params.is_some());

        let account = evm
            .query_account(&QueryAccountRequest {
                address: evm_account.eth_address.clone(),
            })
            .unwrap();
        assert_eq!(account.balance, "12345");
        assert_eq!(account.nonce, 0);

        let balance = evm
            .query_balance(&QueryBalanceRequest {
                address: evm_account.eth_address.clone(),
            })
            .unwrap();
        assert_eq!(balance.balance, "12345");

        let storage = evm
            .query_storage(&QueryStorageRequest {
                address: evm_account.eth_address.clone(),
                key: format!("0x{}", "0".repeat(64)),
            })
            .unwrap();
        assert_eq!(storage.value, format!("0x{}", "0".repeat(64)));

        let code = evm
            .query_code(&QueryCodeRequest {
                address: evm_account.eth_address,
            })
            .unwrap();
        assert!(code.code.is_empty());
    }

    #[test]
    fn execute_raw_ethereum_tx_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let validator_private_key = app.get_first_validator_private_key().unwrap();
        let sender = derive_evm_account(&validator);
        let recipient = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let initial_sender_balance = 1_000_000_000u128;
        let gas_price = 2_500u64;
        let gas_limit = 21_000u64;
        let transfer_value = 777u64;

        fund_evm_account(&bank, &funder, &sender.inj_address, initial_sender_balance);

        let sender_before = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address.clone(),
            })
            .unwrap();
        assert_eq!(sender_before.balance, initial_sender_balance.to_string());
        assert_eq!(sender_before.nonce, 0);

        let raw_tx = build_legacy_signed_raw_tx(
            &validator_private_key,
            evm_chain_id(&evm),
            0,
            gas_price,
            gas_limit,
            parse_evm_address(&recipient.eth_address),
            transfer_value,
            &[],
        );

        let response = evm.execute_raw_ethereum_tx(raw_tx).unwrap();
        assert!(response.data.vm_error.is_empty());
        assert!(!response.data.hash.is_empty());
        assert_eq!(response.data.gas_used, gas_limit);

        let sender_after = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address,
            })
            .unwrap();
        assert_eq!(sender_after.nonce, 1);
        assert_eq!(
            sender_after.balance,
            (initial_sender_balance
                - u128::from(transfer_value)
                - u128::from(gas_price * gas_limit))
            .to_string()
        );

        let recipient_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_after.balance, transfer_value.to_string());
    }

    #[test]
    fn execute_raw_ethereum_txs_batch_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let validator_private_key = app.get_first_validator_private_key().unwrap();
        let sender = derive_evm_account(&validator);
        let recipient_one = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let recipient_two = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let initial_sender_balance = 2_000_000_000u128;
        let gas_price = 2_500u64;
        let gas_limit = 21_000u64;
        let first_value = 111u64;
        let second_value = 222u64;
        let chain_id = evm_chain_id(&evm);

        fund_evm_account(&bank, &funder, &sender.inj_address, initial_sender_balance);

        let first_tx = build_legacy_signed_raw_tx(
            &validator_private_key,
            chain_id,
            0,
            gas_price,
            gas_limit,
            parse_evm_address(&recipient_one.eth_address),
            first_value,
            &[],
        );
        let second_tx = build_legacy_signed_raw_tx(
            &validator_private_key,
            chain_id,
            1,
            gas_price,
            gas_limit,
            parse_evm_address(&recipient_two.eth_address),
            second_value,
            &[],
        );

        let response = evm
            .execute_raw_ethereum_txs(&[first_tx, second_tx])
            .unwrap();
        assert_eq!(response.responses.len(), 2);
        assert!(response.responses.iter().all(|res| res.vm_error.is_empty()));
        assert!(response.gas_info.gas_used > 0);

        let sender_after = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address,
            })
            .unwrap();
        assert_eq!(sender_after.nonce, 2);
        assert_eq!(
            sender_after.balance,
            (initial_sender_balance
                - u128::from(first_value + second_value)
                - u128::from(gas_price * gas_limit * 2))
            .to_string()
        );

        let recipient_one_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_one.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_one_after.balance, first_value.to_string());

        let recipient_two_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_two.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_two_after.balance, second_value.to_string());
    }
}
