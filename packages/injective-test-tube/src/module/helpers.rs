use crate::{Account, Gov, InjectiveTestApp, Insurance, Oracle, Runner, SigningAccount};

use injective_std::{
    shim::Any,
    types::cosmos::{
        base::v1beta1::Coin as BaseCoin,
        gov::v1::{MsgSubmitProposal, MsgVote},
        gov::v1beta1::MsgSubmitProposal as MsgSubmitProposalV1Beta1,
    },
    types::injective::{
        exchange::v2,
        insurance::v1beta1::MsgCreateInsuranceFund,
        oracle::v1beta1::{GrantPriceFeederPrivilegeProposal, MsgRelayPriceFeedPrice, OracleType},
    },
};

use prost::Message;
use std::str::FromStr;
use test_tube_inj::Module;

#[allow(dead_code)]
pub fn add_exchange_admin(
    app: &InjectiveTestApp,
    validator: &SigningAccount,
    admin_address: String,
) {
    let gov = Gov::new(app);
    let res: v2::QueryExchangeParamsResponse = app
        .query(
            "/injective.exchange.v2.Query/QueryExchangeParams",
            &v2::QueryExchangeParamsRequest {},
        )
        .unwrap();

    let mut exchange_params = res.params.unwrap();
    exchange_params.exchange_admins.push(admin_address);
    exchange_params.max_derivative_order_side_count = 300u32;
    exchange_params.post_only_mode_blocks_amount = 1u64;
    exchange_params.post_only_mode_blocks_amount_after_downtime = 1u64;

    // NOTE: this could change int the future
    let governance_module_address = "inj10d07y265gmmuvt4z0w9aw880jnsr700jstypyt";

    let mut buf = vec![];
    v2::MsgUpdateParams::encode(
        &v2::MsgUpdateParams {
            authority: governance_module_address.to_string(),
            params: Some(exchange_params),
        },
        &mut buf,
    )
    .unwrap();

    let res = gov
        .submit_proposal(
            MsgSubmitProposal {
                messages: vec![Any {
                    type_url: v2::MsgUpdateParams::TYPE_URL.to_string(),
                    value: buf,
                }],
                initial_deposit: vec![BaseCoin {
                    amount: "100000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
                proposer: validator.address(),
                metadata: "".to_string(),
                title: "Update params".to_string(),
                summary: "Basically updating the params".to_string(),
                expedited: false,
            },
            validator,
        )
        .unwrap();

    let proposal_id = res
        .events
        .iter()
        .find(|e| e.ty == "submit_proposal")
        .unwrap()
        .attributes[0]
        .value
        .clone();

    gov.vote(
        MsgVote {
            proposal_id: u64::from_str(&proposal_id).unwrap(),
            voter: validator.address(),
            option: 1i32,
            metadata: "".to_string(),
        },
        validator,
    )
    .unwrap();
}

#[allow(dead_code)]
pub fn launch_price_feed_oracle(
    app: &InjectiveTestApp,
    signer: &SigningAccount,
    validator: &SigningAccount,
    base: &str,
    quote: &str,
    dec_price: String,
) {
    let gov = Gov::new(app);
    let oracle = Oracle::new(app);

    let mut buf = vec![];
    GrantPriceFeederPrivilegeProposal::encode(
        &GrantPriceFeederPrivilegeProposal {
            title: "test-proposal".to_string(),
            description: "test-proposal".to_string(),
            base: base.to_string(),
            quote: quote.to_string(),
            relayers: vec![signer.address()],
        },
        &mut buf,
    )
    .unwrap();

    let res = gov
        .submit_proposal_v1beta1(
            MsgSubmitProposalV1Beta1 {
                content: Some(Any {
                    type_url: "/injective.oracle.v1beta1.GrantPriceFeederPrivilegeProposal"
                        .to_string(),
                    value: buf,
                }),
                initial_deposit: vec![BaseCoin {
                    amount: "100000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
                proposer: validator.address(),
            },
            validator,
        )
        .unwrap();

    let proposal_id = res
        .events
        .iter()
        .find(|e| e.ty == "submit_proposal")
        .unwrap()
        .attributes[0]
        .value
        .to_owned();

    gov.vote(
        MsgVote {
            proposal_id: u64::from_str(&proposal_id).unwrap(),
            voter: validator.address(),
            option: 1i32,
            metadata: "".to_string(),
        },
        validator,
    )
    .unwrap();

    // NOTE: increase the block time in order to move past the voting period
    app.increase_time(10u64);

    oracle
        .relay_price_feed(
            MsgRelayPriceFeedPrice {
                sender: signer.address(),
                base: vec![base.to_string()],
                quote: vec![quote.to_string()],
                price: vec![dec_price], // 1.2@18dp
            },
            signer,
        )
        .unwrap();
}

#[allow(dead_code)]
pub fn launch_insurance_fund(
    app: &InjectiveTestApp,
    signer: &SigningAccount,
    ticker: &str,
    quote: &str,
    oracle_base: &str,
    oracle_quote: &str,
    oracle_type: OracleType,
) {
    let insurance = Insurance::new(app);

    insurance
        .create_insurance_fund(
            MsgCreateInsuranceFund {
                sender: signer.address(),
                ticker: ticker.to_string(),
                quote_denom: quote.to_string(),
                oracle_base: oracle_base.to_string(),
                oracle_quote: oracle_quote.to_string(),
                oracle_type: oracle_type as i32,
                expiry: -1i64,
                initial_deposit: Some(BaseCoin {
                    amount: "1000000000".to_string(),
                    denom: quote.to_string(),
                }),
            },
            signer,
        )
        .unwrap();
}
