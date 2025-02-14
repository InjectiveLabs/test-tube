use injective_std::types::cosmos::gov::v1::{
    MsgSubmitProposal, MsgSubmitProposalResponse, MsgVote, MsgVoteResponse, QueryProposalRequest,
    QueryProposalResponse,
};
use injective_std::types::cosmos::gov::v1beta1;
use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;
use test_tube_inj::{fn_execute, fn_query};

pub struct Gov<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Gov<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Gov<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub submit_proposal: MsgSubmitProposal => MsgSubmitProposalResponse
    }

    fn_execute! {
        pub submit_proposal_v1beta1: v1beta1::MsgSubmitProposal => v1beta1::MsgSubmitProposalResponse

    }

    fn_execute! {
        pub vote: MsgVote => MsgVoteResponse
    }

    fn_query! {
        pub query_proposal ["/cosmos.gov.v1beta1.Query/Proposal"]: QueryProposalRequest => QueryProposalResponse
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Coin;
    use injective_std::shim::Any;
    use injective_std::types::{
        cosmos::gov::v1::{MsgSubmitProposal, MsgVote},
        cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin as SDKCoin},
        injective::exchange::v1beta1,
    };
    use prost::Message;
    use std::str::FromStr;

    use crate::{Account, Bank, Gov, InjectiveTestApp, Runner};
    use test_tube_inj::Module;

    #[test]
    fn gov_integration() {
        let app = InjectiveTestApp::new();

        let bank = Bank::new(&app);

        let signer = app
            .init_account(&[
                Coin::new(10_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2f64)
            .unwrap();

        let owner = app
            .init_account(&[
                Coin::new(10_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: validator.address(),
                amount: vec![SDKCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();

        let gov = Gov::new(&app);

        let res: v1beta1::QueryExchangeParamsResponse = app
            .query(
                "/injective.exchange.v1beta1.Query/QueryExchangeParams",
                &v1beta1::QueryExchangeParamsRequest {},
            )
            .unwrap();

        let mut exchange_params = res.params.unwrap();
        exchange_params.exchange_admins.push(owner.address());
        exchange_params.max_derivative_order_side_count = 300u32;

        // NOTE: this could change int he future
        let governance_module_address = "inj10d07y265gmmuvt4z0w9aw880jnsr700jstypyt";

        let mut buf = vec![];
        v1beta1::MsgUpdateParams::encode(
            &v1beta1::MsgUpdateParams {
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
                        type_url: v1beta1::MsgUpdateParams::TYPE_URL.to_string(),
                        value: buf,
                    }],
                    initial_deposit: vec![SDKCoin {
                        amount: "100000000000000000000".to_string(),
                        denom: "inj".to_string(),
                    }],
                    proposer: validator.address(),
                    metadata: "".to_string(),
                    title: "Update params".to_string(),
                    summary: "Basically updating the params".to_string(),
                    expedited: false,
                },
                &validator,
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
            &validator,
        )
        .unwrap();
    }
}
