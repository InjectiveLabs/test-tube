use injective_std::types::injective::auction::v1beta1::{
    QueryAuctionParamsRequest, QueryAuctionParamsResponse, QueryCurrentAuctionBasketRequest,
    QueryCurrentAuctionBasketResponse, QueryLastAuctionResultRequest,
    QueryLastAuctionResultResponse, QueryModuleStateRequest, QueryModuleStateResponse,
};
use test_tube_inj::{fn_query, module::Module, runner::Runner};

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
    use crate::{Auction, Exchange, InjectiveTestApp};
    use cosmwasm_std::Coin;
    use injective_std::types::injective::exchange::v1beta1::MsgDeposit;
    use injective_std::types::{
        cosmos::base::v1beta1::Coin as BaseCoin,
        injective::auction::v1beta1::{
            LastAuctionResult, Params, QueryAuctionParamsRequest, QueryCurrentAuctionBasketRequest,
            QueryLastAuctionResultRequest,
        },
        injective::exchange::v1beta1::QueryDenomDecimalsRequest,
    };
    use test_tube_inj::{Account, Module};

    #[test]
    fn auction_integration() {
        let app = InjectiveTestApp::new();
        let exchange = Exchange::new(&app);
        let auction = Auction::new(&app);

        let response = auction
            .query_auction_params(&QueryAuctionParamsRequest {})
            .unwrap();
        assert_eq!(
            response.params,
            Some(Params {
                auction_period: 604800,
                min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string(),
                inj_basket_max_cap: "10000000000000000000000".to_string(),
                bidders_whitelist: vec![],
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

        let block_time_sec = app.get_block_time_seconds() as u64;

        let basket_res = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .expect("query_current_auction_basket should succeed");

        let closing_time = basket_res.auction_closing_time;
        let round = basket_res.auction_round;

        assert_eq!(round, 0, "Round should be 0");
        assert!(closing_time > 0, "closing_time should be positive");
        assert!(
            closing_time > block_time_sec,
            "closing_time ({}) should be bigger than block_time_sec ({})",
            closing_time,
            block_time_sec
        );

        app.increase_time(1);

        let basket_response_after_increase = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .expect("query_current_auction_basket should succeed (after)");

        assert_eq!(
            basket_response_after_increase.auction_round, round,
            "Round should not change"
        );

        assert_eq!(
            basket_response_after_increase.auction_closing_time,
            closing_time,
        );

        app.increase_time(closing_time - block_time_sec + 1);

        let basket_response_after_increase = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .expect("query_current_auction_basket should succeed (after)");
        assert!(
            basket_response_after_increase.auction_round > round,
            "Round should increase"
        );

        assert!(
            basket_response_after_increase.auction_closing_time > closing_time,
            "Closing time should increase"
        );

        // validate if coins on basket
        let decimals = exchange
            .query_denom_decimals(&QueryDenomDecimalsRequest {
                denoms: vec!["inj".to_string(), "usdt".to_string()],
            })
            .unwrap();

        assert_eq!(decimals.denom_decimals.len(), 2);

        let auction_subaccount = "1111111111111111111111111111111111111111111111111111111111111111";
        let trader = app
            .init_account(&[
                Coin::new(10_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        let _ = exchange
            .deposit(
                MsgDeposit {
                    sender: trader.address().to_string(),
                    subaccount_id: auction_subaccount.to_string(),
                    amount: Some(BaseCoin {
                        denom: "inj".to_string(),
                        amount: "1000000000000000000000".to_string(),
                    }),
                },
                &trader,
            )
            .unwrap();

        let block_time_sec = app.get_block_time_seconds() as u64;
        app.increase_time(basket_response_after_increase.auction_closing_time - block_time_sec + 1);

        let basket_res = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .expect("query_current_auction_basket should succeed");

        assert_eq!(basket_res.amount.len(), 1);
        assert_eq!(basket_res.auction_round, 2);
    }
}
