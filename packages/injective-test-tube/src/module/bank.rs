use injective_std::types::cosmos::bank::v1beta1::{
    MsgSend, MsgSendResponse, QueryAllBalancesRequest, QueryAllBalancesResponse,
    QueryBalanceRequest, QueryBalanceResponse, QueryDenomMetadataRequest,
    QueryDenomMetadataResponse, QueryTotalSupplyRequest, QueryTotalSupplyResponse,
};
use test_tube_inj::{fn_execute, fn_query};

use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;

pub struct Bank<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Bank<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Bank<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub send: MsgSend["/cosmos.bank.v1beta1.MsgSend"] => MsgSendResponse
    }

    fn_query! {
        pub query_balance ["/cosmos.bank.v1beta1.Query/Balance"]: QueryBalanceRequest => QueryBalanceResponse
    }

    fn_query! {
        pub query_all_balances ["/cosmos.bank.v1beta1.Query/AllBalances"]: QueryAllBalancesRequest => QueryAllBalancesResponse
    }

    fn_query! {
        pub query_total_supply ["/cosmos.bank.v1beta1.Query/TotalSupply"]: QueryTotalSupplyRequest => QueryTotalSupplyResponse
    }

    fn_query! {
        pub query_denom_metadata ["/cosmos.bank.v1beta1.Query/DenomMetadata"]: QueryDenomMetadataRequest => QueryDenomMetadataResponse
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Coin;
    use injective_std::types::cosmos::bank::v1beta1::{
        MsgSend, QueryBalanceRequest, QueryDenomMetadataRequest,
    };
    use injective_std::types::cosmos::base::v1beta1::Coin as BaseCoin;

    use crate::{Account, Bank, InjectiveTestApp};
    use test_tube_inj::Module;

    #[test]
    fn bank_integration() {
        let app = InjectiveTestApp::new();
        let signer = app
            .init_account(&[Coin::new(100_000_000_000_000_000_000u128, "inj")])
            .unwrap();
        let receiver = app.init_account(&[Coin::new(1u128, "inj")]).unwrap();
        let bank = Bank::new(&app);

        let response = bank
            .query_balance(&QueryBalanceRequest {
                address: receiver.address(),
                denom: "inj".to_string(),
            })
            .unwrap();
        assert_eq!(
            response.balance.unwrap(),
            BaseCoin {
                amount: 1u128.to_string(),
                denom: "inj".to_string(),
            }
        );

        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: receiver.address(),
                amount: vec![BaseCoin {
                    amount: 9u128.to_string(),
                    denom: "inj".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();
    }

    #[test]
    fn bank_integration_with_decimals() {
        let app = InjectiveTestApp::new();
        app.init_account_decimals(
            &[
                Coin::new(100_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdc"),
                Coin::new(100_000_000_000_000_000_000u128, "atom"),
            ],
            &[18u32, 6u32, 8u32],
        )
        .unwrap();
        let bank = Bank::new(&app);

        let inj_decimals = bank
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: "inj".to_string(),
            })
            .unwrap()
            .metadata
            .unwrap()
            .decimals;
        assert_eq!(inj_decimals, 18);

        let usdc_decimals = bank
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: "usdc".to_string(),
            })
            .unwrap()
            .metadata
            .unwrap()
            .decimals;
        assert_eq!(usdc_decimals, 6);

        let atom_decimals = bank
            .query_denom_metadata(&QueryDenomMetadataRequest {
                denom: "atom".to_string(),
            })
            .unwrap()
            .metadata
            .unwrap()
            .decimals;
        assert_eq!(atom_decimals, 8);
    }
}
