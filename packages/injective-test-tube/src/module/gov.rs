use injective_std::types::cosmos::gov::{v1, v1beta1};
use test_tube_inj::{fn_execute, fn_query, module::Module, runner::Runner};

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
        pub submit_proposal: v1::MsgSubmitProposal => v1::MsgSubmitProposalResponse
    }

    fn_execute! {
        pub submit_proposal_v1beta1: v1beta1::MsgSubmitProposal => v1beta1::MsgSubmitProposalResponse

    }

    fn_execute! {
        pub vote: v1::MsgVote => v1::MsgVoteResponse
    }

    fn_execute! {
        pub vote_v1beta1: v1beta1::MsgVote => v1beta1::MsgVoteResponse
    }

    fn_query! {
        pub query_proposal ["/cosmos.gov.v1beta1.Query/Proposal"]: v1::QueryProposalRequest => v1::QueryProposalResponse
    }
    fn_query! {
        pub query_proposal_v1beta1 ["/cosmos.gov.v1beta1.Query/Proposal"]: v1beta1::QueryProposalRequest => v1beta1::QueryProposalResponse
    }
}
