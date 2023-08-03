// use injective_std::types::cosmos::bank::v1beta1::{
//     MsgSend, MsgSendResponse, QueryAllBalancesRequest, QueryAllBalancesResponse,
//     QueryBalanceRequest, QueryBalanceResponse, QueryTotalSupplyRequest, QueryTotalSupplyResponse,
// };
use injective_std::types::cosmos::staking::v1beta1::{
    MsgBeginRedelegate, MsgBeginRedelegateResponse, MsgCancelUnbondingDelegation,
    MsgCancelUnbondingDelegationResponse, MsgCreateValidator, MsgCreateValidatorResponse,
    MsgDelegate, MsgDelegateResponse, MsgEditValidator, MsgEditValidatorResponse, MsgUndelegate,
    MsgUndelegateResponse,
};
use test_tube_inj::fn_execute;

use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;

pub struct Staking<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Staking<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Staking<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub delegate: MsgDelegate["/cosmos.staking.v1beta1.MsgDelegate"] => MsgDelegateResponse
    }

    fn_execute! {
        pub undelegate: MsgUndelegate["/cosmos.staking.v1beta1.MsgUndelegate"] => MsgUndelegateResponse
    }

    fn_execute! {
        pub create_delegator: MsgCreateValidator["/cosmos.staking.v1beta1.MsgCreateValidator"] => MsgCreateValidatorResponse
    }

    fn_execute! {
        pub edit_delegator: MsgEditValidator["/cosmos.staking.v1beta1.MsgEditValidator"] => MsgEditValidatorResponse
    }

    fn_execute! {
        pub begin_redelegate: MsgBeginRedelegate["/cosmos.staking.v1beta1.MsgBeginRedelegate"] => MsgBeginRedelegateResponse
    }

    fn_execute! {
        pub cancel_unbonding_delegation: MsgCancelUnbondingDelegation["/cosmos.staking.v1beta1.MsgCancelUnbondingDelegation"] => MsgCancelUnbondingDelegationResponse
    }
}
