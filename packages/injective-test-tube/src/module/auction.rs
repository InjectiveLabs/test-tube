use injective_std::types::injective::auction::v1beta1::{
    QueryAuctionParamsRequest, QueryAuctionParamsResponse, QueryCurrentAuctionBasketRequest,
    QueryCurrentAuctionBasketResponse, QueryLastAuctionResultRequest,
    QueryLastAuctionResultResponse, QueryModuleStateRequest, QueryModuleStateResponse,
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

    use crate::{Auction, InjectiveTestApp};
    use test_tube_inj::Module;

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
                min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string(),
                inj_basket_max_cap: "".to_string(),
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
                amount: Some(BaseCoin {
                    denom: "inj".to_string(),
                    amount: "0".to_string()
                }),
                winner: "".to_string(),
                round: 0u64,
            }
        );
    }
}
