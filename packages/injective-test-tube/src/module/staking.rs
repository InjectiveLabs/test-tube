use injective_std::types::cosmos::staking::v1beta1::{
    MsgDelegate, MsgDelegateResponse, MsgUndelegate, MsgUndelegateResponse,
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
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Coin as CosmCoin;
    use injective_std::types::cosmos::{
        base::v1beta1::Coin, staking::v1beta1::MsgDelegate, staking::v1beta1::MsgUndelegate,
    };
    use test_tube_inj::{Account, Module};

    use crate::{InjectiveTestApp, Staking};

    const INJ: &str = "inj";

    #[test]
    fn it_can_delegate_and_undelegate() {
        let app = InjectiveTestApp::new();
        let signer = app
            .init_account(&[CosmCoin::new(100_000_000_000_000_000_000u128, INJ)])
            .unwrap();

        let validator_address = app.get_first_validator_address().unwrap();

        let staking = Staking::new(&app);
        staking
            .delegate(
                MsgDelegate {
                    delegator_address: signer.address(),
                    validator_address: validator_address.clone(),
                    amount: Some(Coin {
                        amount: "1000".to_string(),
                        denom: INJ.to_string(),
                    }),
                },
                &signer,
            )
            .unwrap();

        staking
            .undelegate(
                MsgUndelegate {
                    delegator_address: signer.address(),
                    validator_address,
                    amount: Some(Coin {
                        amount: "1000".to_string(),
                        denom: INJ.to_string(),
                    }),
                },
                &signer,
            )
            .unwrap();
    }
}
