use std::ffi::CString;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::proto::tendermint::v0_38::abci::ResponseFinalizeBlock;
use cosmrs::tx;
use cosmrs::tx::{Fee, ModeInfo, SignMode, SignerInfo, SignerPublicKey};
use cosmwasm_std::{Coin, Timestamp};
use k256::ecdsa::SigningKey as K256SigningKey;
use prost::Message;
use sha3::{Digest, Keccak256};

use crate::account::{Account, AddressDerivation, FeeSetting, SigningAccount};
use crate::bindings::{
    AccountNumber, AccountSequence, CleanUp, FinalizeBlock, FinalizeBlockEvm, GetBlockHeight,
    GetBlockTime, GetParamSet, GetValidatorAddress, GetValidatorPrivateKey, IncreaseTime,
    InitAccount, InitAccountDecimals, InitAccountDecimalsWithDerivation, InitAccountWithDerivation,
    InitTestEnv, Query, Simulate,
};
use crate::redefine_as_go_string;
use crate::runner::error::{DecodeError, EncodeError, RunnerError};
use crate::runner::result::RawResult;
use crate::runner::result::{RunnerExecuteResult, RunnerResult};
use crate::runner::Runner;

pub const INJECTIVE_MIN_GAS_PRICE: u128 = 2_500;
const INJECTIVE_ETHSECP256K1_TYPE_URL: &str = "/injective.crypto.v1beta1.ethsecp256k1.PubKey";

#[derive(Clone, PartialEq, Message)]
struct InjectiveEthSecp256k1PubKey {
    #[prost(bytes = "vec", tag = "1")]
    key: Vec<u8>,
}

fn decode_signing_key(base64_priv: &str) -> RunnerResult<([u8; 32], SigningKey)> {
    let secp256k1_priv = BASE64_STANDARD
        .decode(base64_priv)
        .map_err(DecodeError::Base64DecodeError)?;

    let private_key_bytes: [u8; 32] =
        secp256k1_priv
            .as_slice()
            .try_into()
            .map_err(|_| DecodeError::SigningKeyDecodeError {
                msg: "expected 32-byte secp256k1 private key".to_string(),
            })?;

    let signing_key = SigningKey::from_slice(&private_key_bytes).map_err(|e| {
        let msg = e.to_string();
        DecodeError::SigningKeyDecodeError { msg }
    })?;

    Ok((private_key_bytes, signing_key))
}

fn build_injective_ethsecp256k1_public_key_any(signer: &SigningAccount) -> cosmrs::Any {
    cosmrs::Any {
        type_url: INJECTIVE_ETHSECP256K1_TYPE_URL.to_string(),
        value: InjectiveEthSecp256k1PubKey {
            key: signer.public_key().to_bytes(),
        }
        .encode_to_vec(),
    }
}

fn build_signer_info(signer: &SigningAccount, sequence: u64) -> SignerInfo {
    match signer.derivation() {
        AddressDerivation::Cosmos => SignerInfo::single_direct(Some(signer.public_key()), sequence),
        AddressDerivation::InjectiveEvm => SignerInfo {
            public_key: Some(SignerPublicKey::Any(
                build_injective_ethsecp256k1_public_key_any(signer),
            )),
            mode_info: ModeInfo::single(SignMode::Direct),
            sequence,
        },
    }
}

fn sign_tx_raw(sign_doc: tx::SignDoc, signer: &SigningAccount) -> RunnerResult<tx::Raw> {
    match signer.derivation() {
        AddressDerivation::Cosmos => sign_doc.sign(signer.signing_key()).map_err(|err| {
            RunnerError::GenericError(format!("failed to sign Cosmos transaction: {err}"))
        }),
        AddressDerivation::InjectiveEvm => {
            let signing_key =
                K256SigningKey::from_slice(signer.private_key_bytes()).map_err(|err| {
                    RunnerError::GenericError(format!("invalid secp256k1 private key: {err}"))
                })?;

            let sign_doc_bytes = sign_doc.clone().into_bytes().map_err(RunnerError::from)?;
            let mut digest = Keccak256::new();
            digest.update(sign_doc_bytes);

            let (signature, _) = signing_key.sign_digest_recoverable(digest).map_err(|err| {
                RunnerError::GenericError(format!(
                    "failed to sign Injective ethsecp256k1 transaction: {err}"
                ))
            })?;

            Ok(cosmrs::proto::cosmos::tx::v1beta1::TxRaw {
                body_bytes: sign_doc.body_bytes,
                auth_info_bytes: sign_doc.auth_info_bytes,
                signatures: vec![signature.to_bytes().to_vec()],
            }
            .into())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BaseApp {
    id: u64,
    fee_denom: String,
    chain_id: String,
    address_prefix: String,
    default_gas_adjustment: f64,
}

impl BaseApp {
    pub fn new(
        fee_denom: &str,
        chain_id: &str,
        address_prefix: &str,
        default_gas_adjustment: f64,
    ) -> Self {
        let id = unsafe { InitTestEnv() };
        BaseApp {
            id,
            fee_denom: fee_denom.to_string(),
            chain_id: chain_id.to_string(),
            address_prefix: address_prefix.to_string(),
            default_gas_adjustment,
        }
    }

    /// Increase the time of the blockchain by the given number of seconds.
    pub fn increase_time(&self, seconds: u64) {
        unsafe {
            IncreaseTime(self.id, seconds.try_into().unwrap());
        }
    }

    /// Get the first validator address
    pub fn get_first_validator_address(&self) -> RunnerResult<String> {
        let addr = unsafe {
            let addr = GetValidatorAddress(self.id, 0);
            CString::from_raw(addr)
        }
        .to_str()
        .map_err(DecodeError::Utf8Error)?
        .to_string();

        Ok(addr)
    }

    /// Get the first validator private key
    pub fn get_first_validator_private_key(&self) -> RunnerResult<String> {
        let pkey = unsafe {
            let pkey = GetValidatorPrivateKey(self.id, 0);
            CString::from_raw(pkey)
        }
        .to_str()
        .map_err(DecodeError::Utf8Error)?
        .to_string();

        Ok(pkey)
    }

    /// Get the first validator signing account
    pub fn get_first_validator_signing_account(
        &self,
        denom: String,
        gas_adjustment: f64,
    ) -> RunnerResult<SigningAccount> {
        let pkey = unsafe {
            let pkey = GetValidatorPrivateKey(self.id, 0);
            CString::from_raw(pkey)
        }
        .to_str()
        .map_err(DecodeError::Utf8Error)?
        .to_string();

        let (private_key_bytes, signing_key) = decode_signing_key(&pkey)?;

        let validator = SigningAccount::new(
            self.address_prefix.clone(),
            signing_key,
            private_key_bytes,
            FeeSetting::Auto {
                gas_price: Coin::new(INJECTIVE_MIN_GAS_PRICE, denom),
                gas_adjustment,
            },
        );

        Ok(validator)
    }

    pub fn get_chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn get_account_sequence(&self, address: &str) -> u64 {
        redefine_as_go_string!(address);
        unsafe { AccountSequence(self.id, address) }
    }

    pub fn get_account_number(&self, address: &str) -> u64 {
        redefine_as_go_string!(address);
        unsafe { AccountNumber(self.id, address) }
    }

    /// Get the current block time
    pub fn get_block_timestamp(&self) -> Timestamp {
        let result = unsafe { GetBlockTime(self.id) };

        Timestamp::from_nanos(result as u64)
    }

    /// Get the current block time
    pub fn get_block_time_nanos(&self) -> i64 {
        unsafe { GetBlockTime(self.id) }
    }

    /// Get the current block height
    pub fn get_block_height(&self) -> i64 {
        unsafe { GetBlockHeight(self.id) }
    }
    /// Initialize account with initial balance of any coins, defining decimals if not created.
    /// This function mints new coins and send to newly created account
    pub fn init_account_decimals(
        &self,
        coins: &[Coin],
        decimals: &[u32],
    ) -> RunnerResult<SigningAccount> {
        self.init_account_decimals_with_derivation(coins, decimals, AddressDerivation::Cosmos)
    }

    /// Initialize account with initial balance of any coins, defining decimals if not created,
    /// using an explicit address derivation mode.
    pub fn init_account_decimals_with_derivation(
        &self,
        coins: &[Coin],
        decimals: &[u32],
        derivation: AddressDerivation,
    ) -> RunnerResult<SigningAccount> {
        let mut coins = coins.to_vec();
        let mut decimals = decimals.to_vec();

        // Create indices to track original positions
        let mut indices: Vec<usize> = (0..coins.len()).collect();

        // Sort indices based on coin denominations
        indices.sort_by(|&a, &b| coins[a].denom.cmp(&coins[b].denom));

        // Reorder coins and decimals using the sorted indices
        coins = indices.iter().map(|&i| coins[i].clone()).collect();
        decimals = indices.iter().map(|&i| decimals[i]).collect();

        let coins_json = serde_json::to_string(&coins).map_err(EncodeError::JsonEncodeError)?;
        let decimals_json =
            serde_json::to_string(&decimals).map_err(EncodeError::JsonEncodeError)?;
        redefine_as_go_string!(coins_json);
        redefine_as_go_string!(decimals_json);

        let empty_tx = "".to_string();
        redefine_as_go_string!(empty_tx);

        let base64_priv = unsafe {
            let addr = match derivation {
                AddressDerivation::Cosmos => {
                    InitAccountDecimals(self.id, coins_json, decimals_json)
                }
                AddressDerivation::InjectiveEvm => InitAccountDecimalsWithDerivation(
                    self.id,
                    coins_json,
                    decimals_json,
                    derivation as i32,
                ),
            };
            FinalizeBlock(self.id, empty_tx);
            CString::from_raw(addr)
        }
        .to_str()
        .map_err(DecodeError::Utf8Error)?
        .to_string();

        self.build_signing_account_from_base64(base64_priv, derivation)
    }
    /// Initialize account with initial balance of any coins.
    /// This function mints new coins and send to newly created account
    pub fn init_account(&self, coins: &[Coin]) -> RunnerResult<SigningAccount> {
        self.init_account_with_derivation(coins, AddressDerivation::Cosmos)
    }

    /// Initialize account with initial balance of any coins using an explicit
    /// address derivation mode.
    pub fn init_account_with_derivation(
        &self,
        coins: &[Coin],
        derivation: AddressDerivation,
    ) -> RunnerResult<SigningAccount> {
        let mut coins = coins.to_vec();

        // invalid coins if denom are unsorted
        coins.sort_by(|a, b| a.denom.cmp(&b.denom));

        let coins_json = serde_json::to_string(&coins).map_err(EncodeError::JsonEncodeError)?;
        redefine_as_go_string!(coins_json);

        let empty_tx = "".to_string();
        redefine_as_go_string!(empty_tx);

        let base64_priv = unsafe {
            let addr = match derivation {
                AddressDerivation::Cosmos => InitAccount(self.id, coins_json),
                AddressDerivation::InjectiveEvm => {
                    InitAccountWithDerivation(self.id, coins_json, derivation as i32)
                }
            };
            FinalizeBlock(self.id, empty_tx);
            CString::from_raw(addr)
        }
        .to_str()
        .map_err(DecodeError::Utf8Error)?
        .to_string();

        self.build_signing_account_from_base64(base64_priv, derivation)
    }
    /// Convenience function to create multiple accounts with the same
    /// Initial coins balance
    pub fn init_accounts(&self, coins: &[Coin], count: u64) -> RunnerResult<Vec<SigningAccount>> {
        (0..count).map(|_| self.init_account(coins)).collect()
    }

    fn build_signing_account_from_base64(
        &self,
        base64_priv: String,
        derivation: AddressDerivation,
    ) -> RunnerResult<SigningAccount> {
        let (private_key_bytes, signing_key) = decode_signing_key(&base64_priv)?;

        Ok(SigningAccount::new_with_derivation(
            self.address_prefix.clone(),
            signing_key,
            private_key_bytes,
            derivation,
            FeeSetting::Auto {
                gas_price: Coin::new(INJECTIVE_MIN_GAS_PRICE, self.fee_denom.clone()),
                gas_adjustment: self.default_gas_adjustment,
            },
        ))
    }

    fn create_signed_tx<I>(
        &self,
        msgs: I,
        signer: &SigningAccount,
        fee: Fee,
    ) -> RunnerResult<Vec<u8>>
    where
        I: IntoIterator<Item = cosmrs::Any>,
    {
        let tx_body = tx::Body::new(msgs, "", 0u32);
        let addr = signer.address();

        redefine_as_go_string!(addr);

        let seq = unsafe { AccountSequence(self.id, addr) };
        let account_number = unsafe { AccountNumber(self.id, addr) };

        let signer_info = build_signer_info(signer, seq);

        let chain_id = self
            .chain_id
            .parse()
            .expect("parse const str of chain id should never fail");

        let auth_info = signer_info.auth_info(fee);
        let sign_doc =
            tx::SignDoc::new(&tx_body, &auth_info, &chain_id, account_number).map_err(|e| {
                match e.downcast::<prost::EncodeError>() {
                    Ok(encode_err) => EncodeError::ProtoEncodeError(encode_err),
                    Err(e) => panic!("expect `prost::EncodeError` but got {:?}", e),
                }
            })?;

        let tx_raw = sign_tx_raw(sign_doc, signer)?;

        tx_raw
            .to_bytes()
            .map_err(|e| match e.downcast::<prost::EncodeError>() {
                Ok(encode_err) => EncodeError::ProtoEncodeError(encode_err),
                Err(e) => panic!("expect `prost::EncodeError` but got {:?}", e),
            })
            .map_err(RunnerError::EncodeError)
    }

    pub fn simulate_tx<I>(
        &self,
        msgs: I,
        signer: &SigningAccount,
    ) -> RunnerResult<cosmrs::proto::cosmos::base::abci::v1beta1::GasInfo>
    where
        I: IntoIterator<Item = cosmrs::Any>,
    {
        let tx = self.create_signed_tx(msgs, signer, self.default_simulation_fee())?;
        let base64_tx_bytes = BASE64_STANDARD.encode(tx);

        redefine_as_go_string!(base64_tx_bytes);

        unsafe {
            let res = Simulate(self.id, base64_tx_bytes);
            let res = RawResult::from_non_null_ptr(res).into_result()?;

            cosmrs::proto::cosmos::base::abci::v1beta1::GasInfo::decode(res.as_slice())
                .map_err(DecodeError::ProtoDecodeError)
                .map_err(RunnerError::DecodeError)
        }
    }

    pub fn default_simulation_fee(&self) -> Fee {
        Fee::from_amount_and_gas(
            cosmrs::Coin {
                denom: self.fee_denom.parse().unwrap(),
                amount: INJECTIVE_MIN_GAS_PRICE,
            },
            0u64,
        )
    }

    fn estimate_fee<I>(&self, msgs: I, signer: &SigningAccount) -> RunnerResult<Fee>
    where
        I: IntoIterator<Item = cosmrs::Any>,
    {
        let res = match &signer.fee_setting() {
            FeeSetting::Auto {
                gas_price,
                gas_adjustment,
            } => {
                let gas_info = self.simulate_tx(msgs, signer)?;
                let gas_limit = ((gas_info.gas_used as f64) * (gas_adjustment)).ceil() as u64;

                let amount = cosmrs::Coin {
                    denom: self.fee_denom.parse().unwrap(),
                    amount: (((gas_limit as f64)
                        * (gas_price.amount.to_string().parse::<u128>().unwrap() as f64))
                        .ceil() as u64)
                        .into(),
                };
                Ok(Fee::from_amount_and_gas(amount, gas_limit))
            }
            FeeSetting::Custom { .. } => {
                panic!("estimate fee is a private function and should never be called when fee_setting is Custom");
            }
        };

        res
    }

    /// Get parameter set for a given subspace.
    pub fn get_param_set<P: Message + Default>(
        &self,
        subspace: &str,
        type_url: &str,
    ) -> RunnerResult<P> {
        unsafe {
            redefine_as_go_string!(subspace);
            redefine_as_go_string!(type_url);
            let pset = GetParamSet(self.id, subspace, type_url);
            let pset = RawResult::from_non_null_ptr(pset).into_result()?;
            let pset = P::decode(pset.as_slice()).map_err(DecodeError::ProtoDecodeError)?;
            Ok(pset)
        }
    }

    pub fn execute_signed_evm_txs_raw_response(
        &self,
        raw_txs: &[Vec<u8>],
    ) -> RunnerResult<ResponseFinalizeBlock> {
        let base64_raw_txs = raw_txs
            .iter()
            .map(|raw_tx| BASE64_STANDARD.encode(raw_tx))
            .collect::<Vec<_>>();

        let base64_raw_txs_json =
            serde_json::to_string(&base64_raw_txs).map_err(EncodeError::JsonEncodeError)?;
        redefine_as_go_string!(base64_raw_txs_json);

        unsafe {
            let res = FinalizeBlockEvm(self.id, base64_raw_txs_json);
            let res = RawResult::from_non_null_ptr(res).into_result()?;

            ResponseFinalizeBlock::decode(res.as_slice())
                .map_err(DecodeError::ProtoDecodeError)
                .map_err(RunnerError::DecodeError)
        }
    }
}

/// Cleanup the test environment when the app is dropped.
impl Drop for BaseApp {
    fn drop(&mut self) {
        unsafe {
            CleanUp(self.id);
        }
    }
}

impl<'a> Runner<'a> for BaseApp {
    fn execute_multiple<M, R>(
        &self,
        msgs: &[(M, &str)],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<R>
    where
        M: ::prost::Message,
        R: ::prost::Message + Default,
    {
        let msgs = msgs
            .iter()
            .map(|(msg, type_url)| {
                let mut buf = Vec::new();
                M::encode(msg, &mut buf).map_err(EncodeError::ProtoEncodeError)?;

                Ok(cosmrs::Any {
                    type_url: type_url.to_string(),
                    value: buf,
                })
            })
            .collect::<Result<Vec<cosmrs::Any>, RunnerError>>()?;

        self.execute_multiple_raw(msgs, signer)
    }

    fn execute_multiple_raw<R>(
        &self,
        msgs: Vec<cosmrs::Any>,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<R>
    where
        R: ::prost::Message + Default,
    {
        unsafe {
            let fee = match &signer.fee_setting() {
                FeeSetting::Auto { .. } => self.estimate_fee(msgs.clone(), signer)?,
                FeeSetting::Custom { amount, gas_limit } => Fee::from_amount_and_gas(
                    cosmrs::Coin {
                        denom: amount.denom.parse().unwrap(),
                        amount: amount.amount.to_string().parse().unwrap(),
                    },
                    *gas_limit,
                ),
            };

            let tx = self.create_signed_tx(msgs.clone(), signer, fee)?;
            let base64_tx_bytes = BASE64_STANDARD.encode(tx);

            redefine_as_go_string!(base64_tx_bytes);

            let res = FinalizeBlock(self.id, base64_tx_bytes);
            let res = RawResult::from_non_null_ptr(res).into_result()?;

            let res = ResponseFinalizeBlock::decode(res.as_slice())
                .unwrap()
                .try_into();

            res
        }
    }

    fn query<Q, R>(&self, path: &str, q: &Q) -> RunnerResult<R>
    where
        Q: ::prost::Message,
        R: ::prost::Message + Default,
    {
        let mut buf = Vec::new();

        Q::encode(q, &mut buf).map_err(EncodeError::ProtoEncodeError)?;

        let base64_query_msg_bytes = BASE64_STANDARD.encode(buf);

        redefine_as_go_string!(path);
        redefine_as_go_string!(base64_query_msg_bytes);

        unsafe {
            let res = Query(self.id, path, base64_query_msg_bytes);
            let res = RawResult::from_non_null_ptr(res).into_result()?;
            R::decode(res.as_slice())
                .map_err(DecodeError::ProtoDecodeError)
                .map_err(RunnerError::DecodeError)
        }
    }
}
