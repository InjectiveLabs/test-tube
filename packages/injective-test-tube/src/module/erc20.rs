use injective_std::types::injective::erc20::v1beta1::{
    QueryAllTokenPairsRequest, QueryAllTokenPairsResponse, QueryParamsRequest, QueryParamsResponse,
    QueryTokenPairByDenomRequest, QueryTokenPairByDenomResponse,
    QueryTokenPairByErc20AddressRequest, QueryTokenPairByErc20AddressResponse,
};
use test_tube_inj::{fn_query, module::Module, runner::Runner};

pub struct Erc20<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Erc20<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Erc20<'a, R>
where
    R: Runner<'a>,
{
    fn_query! {
        pub query_params ["/injective.erc20.v1beta1.Query/Params"]: QueryParamsRequest => QueryParamsResponse
    }

    fn_query! {
        pub query_all_token_pairs ["/injective.erc20.v1beta1.Query/AllTokenPairs"]: QueryAllTokenPairsRequest => QueryAllTokenPairsResponse
    }

    fn_query! {
        pub query_token_pair_by_denom ["/injective.erc20.v1beta1.Query/TokenPairByDenom"]: QueryTokenPairByDenomRequest => QueryTokenPairByDenomResponse
    }

    fn_query! {
        pub query_token_pair_by_erc20_address ["/injective.erc20.v1beta1.Query/TokenPairByERC20Address"]: QueryTokenPairByErc20AddressRequest => QueryTokenPairByErc20AddressResponse
    }
}

#[cfg(test)]
mod tests {
    use injective_std::types::injective::erc20::v1beta1::{
        QueryAllTokenPairsRequest, QueryParamsRequest, QueryTokenPairByDenomRequest,
        QueryTokenPairByErc20AddressRequest,
    };
    use test_tube_inj::Module;

    use crate::{Erc20, InjectiveTestApp};

    #[test]
    fn erc20_integration() {
        let app = InjectiveTestApp::new();
        let erc20 = Erc20::new(&app);

        let params = erc20.query_params(&QueryParamsRequest {}).unwrap();
        assert!(params.params.is_some());

        let all_pairs = erc20
            .query_all_token_pairs(&QueryAllTokenPairsRequest { pagination: None })
            .unwrap();

        if let Some(token_pair) = all_pairs.token_pairs.first() {
            let by_denom = erc20
                .query_token_pair_by_denom(&QueryTokenPairByDenomRequest {
                    bank_denom: token_pair.bank_denom.clone(),
                })
                .unwrap();
            assert_eq!(by_denom.token_pair.as_ref(), Some(token_pair));

            let by_erc20 = erc20
                .query_token_pair_by_erc20_address(&QueryTokenPairByErc20AddressRequest {
                    erc20_address: token_pair.erc20_address.clone(),
                })
                .unwrap();
            assert_eq!(by_erc20.token_pair.as_ref(), Some(token_pair));
        } else {
            let by_denom = erc20
                .query_token_pair_by_denom(&QueryTokenPairByDenomRequest {
                    bank_denom: "inj/nonexistent".to_string(),
                })
                .unwrap();
            assert!(by_denom.token_pair.is_none());

            let by_erc20 = erc20
                .query_token_pair_by_erc20_address(&QueryTokenPairByErc20AddressRequest {
                    erc20_address: "0x0000000000000000000000000000000000000001".to_string(),
                })
                .unwrap();
            assert!(by_erc20.token_pair.is_none());
        }
    }
}
