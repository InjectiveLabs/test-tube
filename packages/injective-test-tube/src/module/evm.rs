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
use k256::{ecdsa::SigningKey as K256SigningKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey};
use prost::Message;
use rlp::RlpStream;
use serde_json::{Map, Value};
use sha3::{Digest, Keccak256};
use test_tube_inj::{
    account::Account,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvmLegacyTx {
    pub chain_id: Option<u64>,
    pub nonce: u64,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub to: Option<String>,
    pub value: u128,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvmAccessListItem {
    pub address: String,
    pub storage_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvmAccessListTx {
    pub chain_id: Option<u64>,
    pub nonce: u64,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub to: Option<String>,
    pub value: u128,
    pub data: Vec<u8>,
    pub access_list: Vec<EvmAccessListItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvmDynamicFeeTx {
    pub chain_id: Option<u64>,
    pub nonce: u64,
    pub gas_tip_cap: u128,
    pub gas_fee_cap: u128,
    pub gas_limit: u64,
    pub to: Option<String>,
    pub value: u128,
    pub data: Vec<u8>,
    pub access_list: Vec<EvmAccessListItem>,
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
    fn query_eip155_chain_id(&self) -> RunnerResult<u64> {
        self.query_params(&QueryParamsRequest {})?
            .params
            .ok_or(RunnerError::QueryError {
                msg: "missing EVM params".to_string(),
            })?
            .chain_config
            .ok_or(RunnerError::QueryError {
                msg: "missing EVM chain config".to_string(),
            })?
            .eip155_chain_id
            .parse()
            .map_err(|err| RunnerError::GenericError(format!("invalid EVM chain id: {err}")))
    }

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

    pub fn sign_legacy_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmLegacyTx,
    ) -> RunnerResult<Vec<u8>> {
        let chain_id = match tx.chain_id {
            Some(chain_id) => chain_id,
            None => self.query_eip155_chain_id()?,
        };

        sign_legacy_raw_tx(signer.private_key_bytes(), tx, chain_id)
    }

    pub fn sign_access_list_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmAccessListTx,
    ) -> RunnerResult<Vec<u8>> {
        let chain_id = match tx.chain_id {
            Some(chain_id) => chain_id,
            None => self.query_eip155_chain_id()?,
        };

        sign_access_list_raw_tx(signer.private_key_bytes(), tx, chain_id)
    }

    pub fn sign_dynamic_fee_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmDynamicFeeTx,
    ) -> RunnerResult<Vec<u8>> {
        let chain_id = match tx.chain_id {
            Some(chain_id) => chain_id,
            None => self.query_eip155_chain_id()?,
        };

        sign_dynamic_fee_raw_tx(signer.private_key_bytes(), tx, chain_id)
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

pub fn derive_evm_address<A: Account>(account: &A) -> String {
    format_bytes(&derive_evm_address_bytes(account))
}

pub fn derive_injective_evm_address<A: Account>(account: &A) -> String {
    cosmrs::AccountId::new("inj", &derive_evm_address_bytes(account))
        .expect("derived EVM bytes should form a valid Injective address")
        .to_string()
}

fn derive_evm_address_bytes<A: Account>(account: &A) -> [u8; 20] {
    let pubkey_bytes = account.public_key().to_bytes();
    let pubkey = PublicKey::from_sec1_bytes(&pubkey_bytes)
        .expect("account should use a valid secp256k1 public key");
    let uncompressed = pubkey.to_encoded_point(false);
    let hash = Keccak256::digest(&uncompressed.as_bytes()[1..]);

    hash[12..]
        .try_into()
        .expect("Keccak-derived EVM address should contain 20 bytes")
}

fn decode_hex_nibble(ch: u8) -> Result<u8, RunnerError> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        b'A'..=b'F' => Ok(ch - b'A' + 10),
        _ => Err(RunnerError::GenericError(format!(
            "invalid hex character: {}",
            ch as char
        ))),
    }
}

fn decode_hex_bytes(hex: &str) -> RunnerResult<Vec<u8>> {
    let trimmed = hex.strip_prefix("0x").unwrap_or(hex);
    if trimmed.len() % 2 != 0 {
        return Err(RunnerError::GenericError(
            "hex input must have an even length".to_string(),
        ));
    }

    trimmed
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Ok((decode_hex_nibble(pair[0])? << 4) | decode_hex_nibble(pair[1])?))
        .collect()
}

fn parse_evm_address(address: &str) -> RunnerResult<[u8; 20]> {
    decode_hex_bytes(address)?
        .try_into()
        .map_err(|_| RunnerError::GenericError("EVM address should contain 20 bytes".to_string()))
}

fn trim_left_zeroes(bytes: &[u8]) -> Vec<u8> {
    let first_non_zero = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    bytes[first_non_zero..].to_vec()
}

fn append_u128(stream: &mut RlpStream, value: u128) {
    stream.append(&trim_left_zeroes(&value.to_be_bytes()));
}

type ParsedAccessList = Vec<([u8; 20], Vec<[u8; 32]>)>;

fn parse_storage_key(storage_key: &str) -> RunnerResult<[u8; 32]> {
    decode_hex_bytes(storage_key)?
        .try_into()
        .map_err(|_| RunnerError::GenericError("storage key should contain 32 bytes".to_string()))
}

fn parse_access_list(access_list: &[EvmAccessListItem]) -> RunnerResult<ParsedAccessList> {
    access_list
        .iter()
        .map(|item| {
            let address = parse_evm_address(&item.address)?;
            let storage_keys = item
                .storage_keys
                .iter()
                .map(|storage_key| parse_storage_key(storage_key))
                .collect::<Result<Vec<_>, _>>()?;

            Ok((address, storage_keys))
        })
        .collect()
}

fn append_access_list(stream: &mut RlpStream, access_list: &ParsedAccessList) {
    stream.begin_list(access_list.len());
    for (address, storage_keys) in access_list {
        stream.begin_list(2);
        stream.append(&address.to_vec());
        stream.begin_list(storage_keys.len());
        for storage_key in storage_keys {
            stream.append(&storage_key.to_vec());
        }
    }
}

fn sign_legacy_raw_tx(
    private_key_bytes: &[u8; 32],
    tx: &EvmLegacyTx,
    chain_id: u64,
) -> RunnerResult<Vec<u8>> {
    let signing_key = K256SigningKey::from_slice(private_key_bytes).map_err(|err| {
        RunnerError::GenericError(format!("invalid secp256k1 private key bytes: {err}"))
    })?;
    let to = tx.to.as_deref().map(parse_evm_address).transpose()?;

    let mut signing_payload = RlpStream::new_list(9);
    signing_payload.append(&tx.nonce);
    append_u128(&mut signing_payload, tx.gas_price);
    signing_payload.append(&tx.gas_limit);
    match to {
        Some(to) => signing_payload.append(&to.to_vec()),
        None => signing_payload.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signing_payload, tx.value);
    signing_payload.append(&tx.data);
    signing_payload.append(&chain_id);
    signing_payload.append(&0u8);
    signing_payload.append(&0u8);

    let mut digest = Keccak256::new();
    digest.update(signing_payload.out());

    let (signature, recovery_id) = signing_key
        .sign_digest_recoverable(digest)
        .map_err(|err| RunnerError::GenericError(format!("failed to sign legacy tx: {err}")))?;
    let signature_bytes = signature.to_bytes();
    let v = chain_id * 2 + 35 + u64::from(recovery_id.to_byte());

    let mut signed_tx = RlpStream::new_list(9);
    signed_tx.append(&tx.nonce);
    append_u128(&mut signed_tx, tx.gas_price);
    signed_tx.append(&tx.gas_limit);
    match to {
        Some(to) => signed_tx.append(&to.to_vec()),
        None => signed_tx.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signed_tx, tx.value);
    signed_tx.append(&tx.data);
    signed_tx.append(&v);
    signed_tx.append(&trim_left_zeroes(&signature_bytes[..32]));
    signed_tx.append(&trim_left_zeroes(&signature_bytes[32..]));

    Ok(signed_tx.out().to_vec())
}

fn sign_access_list_raw_tx(
    private_key_bytes: &[u8; 32],
    tx: &EvmAccessListTx,
    chain_id: u64,
) -> RunnerResult<Vec<u8>> {
    let signing_key = K256SigningKey::from_slice(private_key_bytes).map_err(|err| {
        RunnerError::GenericError(format!("invalid secp256k1 private key bytes: {err}"))
    })?;
    let to = tx.to.as_deref().map(parse_evm_address).transpose()?;
    let access_list = parse_access_list(&tx.access_list)?;

    let mut signing_payload = RlpStream::new_list(8);
    signing_payload.append(&chain_id);
    signing_payload.append(&tx.nonce);
    append_u128(&mut signing_payload, tx.gas_price);
    signing_payload.append(&tx.gas_limit);
    match to {
        Some(to) => signing_payload.append(&to.to_vec()),
        None => signing_payload.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signing_payload, tx.value);
    signing_payload.append(&tx.data);
    append_access_list(&mut signing_payload, &access_list);

    let mut digest = Keccak256::new();
    digest.update([0x01]);
    digest.update(signing_payload.out());

    let (signature, recovery_id) = signing_key.sign_digest_recoverable(digest).map_err(|err| {
        RunnerError::GenericError(format!("failed to sign access-list tx: {err}"))
    })?;
    let signature_bytes = signature.to_bytes();

    let mut signed_payload = RlpStream::new_list(11);
    signed_payload.append(&chain_id);
    signed_payload.append(&tx.nonce);
    append_u128(&mut signed_payload, tx.gas_price);
    signed_payload.append(&tx.gas_limit);
    match to {
        Some(to) => signed_payload.append(&to.to_vec()),
        None => signed_payload.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signed_payload, tx.value);
    signed_payload.append(&tx.data);
    append_access_list(&mut signed_payload, &access_list);
    signed_payload.append(&u64::from(recovery_id.to_byte()));
    signed_payload.append(&trim_left_zeroes(&signature_bytes[..32]));
    signed_payload.append(&trim_left_zeroes(&signature_bytes[32..]));

    let signed_payload_bytes = signed_payload.out().to_vec();
    let mut raw_tx = Vec::with_capacity(1 + signed_payload_bytes.len());
    raw_tx.push(0x01);
    raw_tx.extend_from_slice(&signed_payload_bytes);

    Ok(raw_tx)
}

fn sign_dynamic_fee_raw_tx(
    private_key_bytes: &[u8; 32],
    tx: &EvmDynamicFeeTx,
    chain_id: u64,
) -> RunnerResult<Vec<u8>> {
    let signing_key = K256SigningKey::from_slice(private_key_bytes).map_err(|err| {
        RunnerError::GenericError(format!("invalid secp256k1 private key bytes: {err}"))
    })?;
    let to = tx.to.as_deref().map(parse_evm_address).transpose()?;
    let access_list = parse_access_list(&tx.access_list)?;

    let mut signing_payload = RlpStream::new_list(9);
    signing_payload.append(&chain_id);
    signing_payload.append(&tx.nonce);
    append_u128(&mut signing_payload, tx.gas_tip_cap);
    append_u128(&mut signing_payload, tx.gas_fee_cap);
    signing_payload.append(&tx.gas_limit);
    match to {
        Some(to) => signing_payload.append(&to.to_vec()),
        None => signing_payload.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signing_payload, tx.value);
    signing_payload.append(&tx.data);
    append_access_list(&mut signing_payload, &access_list);

    let mut digest = Keccak256::new();
    digest.update([0x02]);
    digest.update(signing_payload.out());

    let (signature, recovery_id) = signing_key.sign_digest_recoverable(digest).map_err(|err| {
        RunnerError::GenericError(format!("failed to sign dynamic-fee tx: {err}"))
    })?;
    let signature_bytes = signature.to_bytes();

    let mut signed_payload = RlpStream::new_list(12);
    signed_payload.append(&chain_id);
    signed_payload.append(&tx.nonce);
    append_u128(&mut signed_payload, tx.gas_tip_cap);
    append_u128(&mut signed_payload, tx.gas_fee_cap);
    signed_payload.append(&tx.gas_limit);
    match to {
        Some(to) => signed_payload.append(&to.to_vec()),
        None => signed_payload.append(&Vec::<u8>::new()),
    };
    append_u128(&mut signed_payload, tx.value);
    signed_payload.append(&tx.data);
    append_access_list(&mut signed_payload, &access_list);
    signed_payload.append(&u64::from(recovery_id.to_byte()));
    signed_payload.append(&trim_left_zeroes(&signature_bytes[..32]));
    signed_payload.append(&trim_left_zeroes(&signature_bytes[32..]));

    let signed_payload_bytes = signed_payload.out().to_vec();
    let mut raw_tx = Vec::with_capacity(1 + signed_payload_bytes.len());
    raw_tx.push(0x02);
    raw_tx.extend_from_slice(&signed_payload_bytes);

    Ok(raw_tx)
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
    pub fn execute_access_list_txs(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        txs: &[EvmAccessListTx],
    ) -> RunnerResult<EvmExecuteResponse> {
        let raw_txs = txs
            .iter()
            .map(|tx| self.sign_access_list_tx(signer, tx))
            .collect::<Result<Vec<_>, _>>()?;

        self.execute_raw_ethereum_txs(&raw_txs)
    }

    pub fn execute_access_list_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmAccessListTx,
    ) -> RunnerResult<ExecuteResponse<MsgEthereumTxResponse>> {
        let raw_tx = self.sign_access_list_tx(signer, tx)?;
        self.execute_raw_ethereum_tx(raw_tx)
    }

    pub fn execute_dynamic_fee_txs(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        txs: &[EvmDynamicFeeTx],
    ) -> RunnerResult<EvmExecuteResponse> {
        let raw_txs = txs
            .iter()
            .map(|tx| self.sign_dynamic_fee_tx(signer, tx))
            .collect::<Result<Vec<_>, _>>()?;

        self.execute_raw_ethereum_txs(&raw_txs)
    }

    pub fn execute_dynamic_fee_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmDynamicFeeTx,
    ) -> RunnerResult<ExecuteResponse<MsgEthereumTxResponse>> {
        let raw_tx = self.sign_dynamic_fee_tx(signer, tx)?;
        self.execute_raw_ethereum_tx(raw_tx)
    }

    pub fn execute_legacy_txs(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        txs: &[EvmLegacyTx],
    ) -> RunnerResult<EvmExecuteResponse> {
        let raw_txs = txs
            .iter()
            .map(|tx| self.sign_legacy_tx(signer, tx))
            .collect::<Result<Vec<_>, _>>()?;

        self.execute_raw_ethereum_txs(&raw_txs)
    }

    pub fn execute_legacy_tx(
        &self,
        signer: &test_tube_inj::account::SigningAccount,
        tx: &EvmLegacyTx,
    ) -> RunnerResult<ExecuteResponse<MsgEthereumTxResponse>> {
        let raw_tx = self.sign_legacy_tx(signer, tx)?;
        self.execute_raw_ethereum_tx(raw_tx)
    }

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

    use cosmrs::Any;
    use injective_std::types::injective::evm::v1::{
        QueryAccountRequest, QueryBalanceRequest, QueryCodeRequest, QueryParamsRequest,
        QueryStorageRequest,
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
        Bank, Evm, EvmAccessListItem, EvmAccessListTx, EvmCall, EvmDynamicFeeTx, EvmLegacyTx,
        EvmQueryOptions, InjectiveTestApp, RunnerResult,
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
        DerivedEvmAccount {
            inj_address: super::derive_injective_evm_address(account),
            eth_address: super::derive_evm_address(account),
        }
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

    // Test-only helper: builds runtime bytecode for a minimal contract that always
    // returns the bytes appended after the opcode prefix.
    fn build_runtime_returner_bytecode(return_data: &[u8]) -> Vec<u8> {
        assert!(return_data.len() <= 0xffff, "return data too large");

        let return_data_len = return_data.len() as u16;
        let runtime_prefix = vec![
            0x61,
            (return_data_len >> 8) as u8,
            return_data_len as u8,
            0x61,
            0x00,
            0x0f, // the appended return bytes start right after this 15-byte prefix
            0x60,
            0x00,
            0x39,
            0x61,
            (return_data_len >> 8) as u8,
            return_data_len as u8,
            0x60,
            0x00,
            0xf3,
        ];

        [runtime_prefix, return_data.to_vec()].concat()
    }

    // Test-only helper: wraps the runtime bytecode in initcode so CREATE returns
    // that runtime and stores it as the deployed contract code.
    fn build_deployment_initcode_for_returner(return_data: &[u8]) -> Vec<u8> {
        let runtime_bytecode = build_runtime_returner_bytecode(return_data);
        build_runtime_returner_bytecode(&runtime_bytecode)
    }

    fn decode_hex_address(address: &str) -> [u8; 20] {
        let trimmed = address.strip_prefix("0x").unwrap_or(address);
        let decoded = super::decode_hex_bytes(trimmed).expect("valid hex address");
        decoded.try_into().expect("20-byte address")
    }

    fn predict_contract_address(sender: &str, nonce: u64) -> String {
        let sender = decode_hex_address(sender);
        let mut stream = RlpStream::new_list(2);
        stream.append(&sender.as_slice());
        stream.append(&nonce);

        let hash = Keccak256::digest(stream.out());
        format!("0x{}", hex_lower(&hash[12..]))
    }

    fn hex_lower(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }
        output
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

        let raw_tx = evm
            .sign_legacy_tx(
                &validator,
                &EvmLegacyTx {
                    nonce: 0,
                    gas_price: u128::from(gas_price),
                    gas_limit,
                    to: Some(recipient.eth_address.clone()),
                    value: u128::from(transfer_value),
                    data: Vec::new(),
                    chain_id: None,
                },
            )
            .unwrap();

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
    fn execute_legacy_tx_from_init_account_signer_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
            .unwrap();
        let sender = derive_evm_account(&signer);
        let recipient = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let initial_sender_balance = 1_000_000_000u128;
        let gas_price = 2_500u64;
        let gas_limit = 21_000u64;
        let transfer_value = 888u64;

        assert_ne!(signer.address(), sender.inj_address);

        fund_evm_account(&bank, &funder, &sender.inj_address, initial_sender_balance);

        let sender_before = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address.clone(),
            })
            .unwrap();
        assert_eq!(sender_before.balance, initial_sender_balance.to_string());
        assert_eq!(sender_before.nonce, 0);

        let response = evm
            .execute_legacy_tx(
                &signer,
                &EvmLegacyTx {
                    nonce: 0,
                    gas_price: u128::from(gas_price),
                    gas_limit,
                    to: Some(recipient.eth_address.clone()),
                    value: u128::from(transfer_value),
                    data: Vec::new(),
                    chain_id: None,
                },
            )
            .unwrap();
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

        fund_evm_account(&bank, &funder, &sender.inj_address, initial_sender_balance);

        let response = evm
            .execute_legacy_txs(
                &validator,
                &[
                    EvmLegacyTx {
                        nonce: 0,
                        gas_price: u128::from(gas_price),
                        gas_limit,
                        to: Some(recipient_one.eth_address.clone()),
                        value: u128::from(first_value),
                        data: Vec::new(),
                        chain_id: None,
                    },
                    EvmLegacyTx {
                        nonce: 1,
                        gas_price: u128::from(gas_price),
                        gas_limit,
                        to: Some(recipient_two.eth_address.clone()),
                        value: u128::from(second_value),
                        data: Vec::new(),
                        chain_id: None,
                    },
                ],
            )
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

    #[test]
    fn deploy_contract_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let sender = derive_evm_account(&signer);
        let return_data = vec![0xde, 0xad, 0xbe, 0xef];
        let expected_runtime_bytecode = build_runtime_returner_bytecode(&return_data);
        let deployment_initcode = build_deployment_initcode_for_returner(&return_data);

        fund_evm_account(&bank, &funder, &sender.inj_address, 2_000_000_000u128);

        let sender_before = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address.clone(),
            })
            .unwrap();
        let contract_address = predict_contract_address(&sender.eth_address, sender_before.nonce);

        let response = evm
            .execute_legacy_tx(
                &signer,
                &EvmLegacyTx {
                    nonce: sender_before.nonce,
                    gas_price: 2_500,
                    gas_limit: 500_000,
                    to: None,
                    value: 0,
                    data: deployment_initcode,
                    chain_id: None,
                },
            )
            .unwrap();
        assert!(response.data.vm_error.is_empty());
        assert!(!response.data.hash.is_empty());
        assert!(response.data.gas_used > 0);

        let sender_after = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address.clone(),
            })
            .unwrap();
        assert_eq!(sender_after.nonce, sender_before.nonce + 1);

        let code = evm
            .query_code(&QueryCodeRequest {
                address: contract_address.clone(),
            })
            .unwrap();
        assert_eq!(code.code, expected_runtime_bytecode);

        let chain_id = evm
            .query_params(&QueryParamsRequest {})
            .unwrap()
            .params
            .unwrap()
            .chain_config
            .unwrap()
            .eip155_chain_id
            .parse()
            .unwrap();

        let call_response = evm
            .eth_call(
                &EvmCall {
                    to: Some(contract_address),
                    ..Default::default()
                },
                &EvmQueryOptions {
                    gas_cap: 500_000,
                    proposer_address: None,
                    chain_id,
                    overrides: None,
                },
            )
            .unwrap();
        assert!(call_response.vm_error.is_empty());
        assert_eq!(call_response.ret, return_data);
    }

    #[test]
    fn execute_access_list_tx_with_entries_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let sender = derive_evm_account(&signer);
        let recipient = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );

        fund_evm_account(&bank, &funder, &sender.inj_address, 1_000_000_000u128);

        let response = evm
            .execute_access_list_tx(
                &signer,
                &EvmAccessListTx {
                    nonce: 0,
                    gas_price: 2_500,
                    gas_limit: 50_000,
                    to: Some(recipient.eth_address.clone()),
                    value: 333,
                    data: Vec::new(),
                    access_list: vec![EvmAccessListItem {
                        address: recipient.eth_address.clone(),
                        storage_keys: vec![format!("0x{}", "11".repeat(32))],
                    }],
                    chain_id: None,
                },
            )
            .unwrap();
        assert!(response.data.vm_error.is_empty());
        assert!(!response.data.hash.is_empty());
        assert!(response.data.gas_used > 21_000);

        let sender_after = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address,
            })
            .unwrap();
        assert_eq!(sender_after.nonce, 1);

        let recipient_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_after.balance, "333");
    }

    #[test]
    fn execute_access_list_txs_batch_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let sender = derive_evm_account(&signer);
        let recipient_one = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let recipient_two = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );

        fund_evm_account(&bank, &funder, &sender.inj_address, 2_000_000_000u128);

        let response = evm
            .execute_access_list_txs(
                &signer,
                &[
                    EvmAccessListTx {
                        nonce: 0,
                        gas_price: 2_500,
                        gas_limit: 50_000,
                        to: Some(recipient_one.eth_address.clone()),
                        value: 111,
                        data: Vec::new(),
                        access_list: vec![EvmAccessListItem {
                            address: recipient_one.eth_address.clone(),
                            storage_keys: vec![format!("0x{}", "22".repeat(32))],
                        }],
                        chain_id: None,
                    },
                    EvmAccessListTx {
                        nonce: 1,
                        gas_price: 2_500,
                        gas_limit: 50_000,
                        to: Some(recipient_two.eth_address.clone()),
                        value: 222,
                        data: Vec::new(),
                        access_list: vec![EvmAccessListItem {
                            address: recipient_two.eth_address.clone(),
                            storage_keys: vec![format!("0x{}", "33".repeat(32))],
                        }],
                        chain_id: None,
                    },
                ],
            )
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

        let recipient_one_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_one.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_one_after.balance, "111");

        let recipient_two_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_two.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_two_after.balance, "222");
    }

    #[test]
    fn execute_dynamic_fee_tx_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let sender = derive_evm_account(&signer);
        let recipient = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );

        fund_evm_account(&bank, &funder, &sender.inj_address, 1_000_000_000u128);

        let response = evm
            .execute_dynamic_fee_tx(
                &signer,
                &EvmDynamicFeeTx {
                    nonce: 0,
                    gas_tip_cap: 2_500,
                    gas_fee_cap: 2_500,
                    gas_limit: 21_000,
                    to: Some(recipient.eth_address.clone()),
                    value: 444,
                    data: Vec::new(),
                    access_list: Vec::new(),
                    chain_id: None,
                },
            )
            .unwrap();
        assert!(response.data.vm_error.is_empty());
        assert!(!response.data.hash.is_empty());

        let sender_after = evm
            .query_account(&QueryAccountRequest {
                address: sender.eth_address,
            })
            .unwrap();
        assert_eq!(sender_after.nonce, 1);

        let recipient_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_after.balance, "444");
    }

    #[test]
    fn execute_dynamic_fee_txs_batch_integration() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let evm = Evm::new(&app);
        let funder = app
            .init_account(&[cosmwasm_std::Coin::new(5_000_000_000u128, "inj")])
            .unwrap();
        let signer = app
            .get_first_validator_signing_account("inj".to_string(), 1.2)
            .unwrap();
        let sender = derive_evm_account(&signer);
        let recipient_one = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );
        let recipient_two = derive_evm_account(
            &app.init_account(&[cosmwasm_std::Coin::new(1u128, "inj")])
                .unwrap(),
        );

        fund_evm_account(&bank, &funder, &sender.inj_address, 2_000_000_000u128);

        let response = evm
            .execute_dynamic_fee_txs(
                &signer,
                &[
                    EvmDynamicFeeTx {
                        nonce: 0,
                        gas_tip_cap: 2_000,
                        gas_fee_cap: 2_500,
                        gas_limit: 21_000,
                        to: Some(recipient_one.eth_address.clone()),
                        value: 555,
                        data: Vec::new(),
                        access_list: Vec::new(),
                        chain_id: None,
                    },
                    EvmDynamicFeeTx {
                        nonce: 1,
                        gas_tip_cap: 2_000,
                        gas_fee_cap: 2_500,
                        gas_limit: 21_000,
                        to: Some(recipient_two.eth_address.clone()),
                        value: 666,
                        data: Vec::new(),
                        access_list: Vec::new(),
                        chain_id: None,
                    },
                ],
            )
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

        let recipient_one_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_one.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_one_after.balance, "555");

        let recipient_two_after = evm
            .query_balance(&QueryBalanceRequest {
                address: recipient_two.eth_address,
            })
            .unwrap();
        assert_eq!(recipient_two_after.balance, "666");
    }
}
