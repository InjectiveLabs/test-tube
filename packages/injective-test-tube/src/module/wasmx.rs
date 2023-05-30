use injective_std::types::injective::wasmx::v1;

use test_tube_inj::fn_query;

use test_tube_inj::runner::Runner;

pub struct Wasmx<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> super::Module<'a, R> for Wasmx<'a, R> {
    fn new(runner: &'a R) -> Self {
        Wasmx { runner }
    }
}

impl<'a, R> Wasmx<'a, R>
where
    R: Runner<'a>,
{
    fn_query! {
        pub query_contract_registration_info ["/injective.wasmx.v1.Query/ContractRegistrationInfo"]: v1::QueryContractRegistrationInfoRequest => v1::QueryContractRegistrationInfoResponse
    }
}
