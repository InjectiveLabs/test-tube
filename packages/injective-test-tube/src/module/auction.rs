use injective_std::types::injective::auction::v1beta1::{
    LastAuctionResult, QueryAuctionParamsRequest, QueryAuctionParamsResponse,
    QueryCurrentAuctionBasketRequest, QueryCurrentAuctionBasketResponse,
    QueryLastAuctionResultRequest, QueryLastAuctionResultResponse, QueryModuleStateRequest,
    QueryModuleStateResponse,
};
use test_tube_inj::fn_query;

use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;

pub struct Auction<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Auction<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Auction<'a, R>
where
    R: Runner<'a>,
{
    fn_query! {
        pub query_auction_params ["/injective.auction.v1beta1.Query/AuctionParams"]: QueryAuctionParamsRequest => QueryAuctionParamsResponse
    }

    fn_query! {
        pub query_current_auction_basket ["/injective.auction.v1beta1.Query/CurrentAuctionBasket"]: QueryCurrentAuctionBasketRequest => QueryCurrentAuctionBasketResponse
    }

    fn_query! {
        pub query_module_state ["/injective.auction.v1beta1.Query/ModuleState"]: QueryModuleStateRequest => QueryModuleStateResponse
    }

    fn_query! {
        pub query_last_auction_result ["/injective.auction.v1beta1.Query/LastAuctionResult"]: QueryLastAuctionResultRequest => QueryLastAuctionResultResponse
    }
}

#[cfg(test)]
mod tests {
    use injective_std::types::{
        cosmos::base::v1beta1::Coin as BaseCoin,
        injective::auction::v1beta1::{
            LastAuctionResult, Params, QueryAuctionParamsRequest, QueryLastAuctionResultRequest,
        },
    };

    use crate::{Auction, InjectiveTestApp, Wasm};
    use cosmwasm_schema::cw_serde;
    use test_tube_inj::Module;

    #[cw_serde]
    pub struct InstantiateMsg {}

    #[test]
    fn auction_integration() {
        let app = InjectiveTestApp::new();

        let auction = Auction::new(&app);

        let response = auction
            .query_auction_params(&QueryAuctionParamsRequest {})
            .unwrap();
        assert_eq!(
            response.params,
            Some(Params {
                auction_period: 604800,
                min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string()
            })
        );

        let response = auction
            .query_last_auction_result(&QueryLastAuctionResultRequest {})
            .unwrap();
        assert!(response.last_auction_result.is_some());

        let result = response.last_auction_result.unwrap();
        assert_eq!(
            result,
            LastAuctionResult {
                // amount: "100000inj".to_string(),
                amount: Some(BaseCoin {
                    denom: "inj".to_string(),
                    amount: "100000".to_string()
                }),
                winner: "".to_string(),
                round: 0u64,
            }
        );
    }

    #[test]
    fn ttest_cosmwasmpool_proposal() {
        let app = InjectiveTestApp::default();

        let wasm = Wasm::new(&app);

        let signer = app
            .init_account(&[cosmwasm_std::Coin::new(1000000000000000000u128, "inj")])
            .unwrap();

        // upload cosmwasm pool code and whitelist through proposal
        let wasm_byte_code = std::fs::read(
            "/Users/wandlitz/go/src/github.com/InjectiveLabs/test-tube/packages/injective-test-tube/src/module/artifacts/dummy-aarch64.wasm",
        )
        .unwrap();

        let code_id = wasm
            .store_code(&wasm_byte_code, None, &signer)
            .unwrap()
            .data
            .code_id;

        wasm.instantiate(
            code_id,
            &InstantiateMsg {},
            None,
            Some("no label"),
            &[],
            &signer,
        )
        .unwrap();
    }
}
