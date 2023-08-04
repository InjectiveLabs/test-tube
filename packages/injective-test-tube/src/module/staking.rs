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
        pub create_validator: MsgCreateValidator["/cosmos.staking.v1beta1.MsgCreateValidator"] => MsgCreateValidatorResponse
    }

    fn_execute! {
        pub edit_validator: MsgEditValidator["/cosmos.staking.v1beta1.MsgEditValidator"] => MsgEditValidatorResponse
    }

    fn_execute! {
        pub begin_redelegate: MsgBeginRedelegate["/cosmos.staking.v1beta1.MsgBeginRedelegate"] => MsgBeginRedelegateResponse
    }

    fn_execute! {
        pub cancel_unbonding_delegation: MsgCancelUnbondingDelegation["/cosmos.staking.v1beta1.MsgCancelUnbondingDelegation"] => MsgCancelUnbondingDelegationResponse
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Coin as CosmCoin;
    use injective_std::{
        shim::Any,
        types::cosmos::{
            bank::v1beta1::MsgSend,
            base::v1beta1::Coin,
            staking::v1beta1::MsgUndelegate,
            staking::v1beta1::{CommissionRates, MsgDelegate},
            staking::v1beta1::{Description, MsgCreateValidator, MsgEditValidator},
        },
    };
    use test_tube_inj::{Account, Module, SigningAccount};

    use crate::{Bank, InjectiveTestApp, Staking};
    use ed25519;

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

    #[test]
    fn it_can_create_a_validator() {
        let app = InjectiveTestApp::new();
        let signer = app
            .init_account(&[CosmCoin::new(100_000_000_000_000_000_000_000_000u128, INJ)])
            .unwrap();

        let validator_signing_account = app
            .get_first_validator_signing_account(INJ.to_string(), 0.12f64)
            .unwrap();
        let validator_address = app.get_first_validator_address().unwrap();

        let banker = app
            .init_account(&[CosmCoin::new(100_000_000_000_000_000_000_000_000u128, INJ)])
            .unwrap();

        fund_account_with_some_inj(
            &Bank::new(&app),
            &banker,
            &validator_signing_account.address(),
        );

        let keyPair = app.generate_new_validator_private_pub_key_pair().unwrap();
        println!("keyPair: {:?}", keyPair);

        pkcs8::DecodePublicKey::decode(&keyPair.pub_key.key).unwrap();
        // ed25519::from_slice(&keyPair.pub_key.key).unwrap(

        // let as_any = Any::from(keyPair.pub_key.key);

        let staking = Staking::new(&app);
        staking
            .create_validator(
                MsgCreateValidator {
                    validator_address,
                    description: Some(Description {
                        moniker: "new moniker".to_string(),
                        identity: "new identity".to_string(),
                        website: "new website".to_string(),
                        security_contact: "new security contact".to_string(),
                        details: "new details".to_string(),
                    }),
                    commission: Some(CommissionRates {
                        rate: "500".to_string(),
                        max_rate: "1000".to_string(),
                        max_change_rate: "100".to_string(),
                    }),
                    min_self_delegation: "2000".to_string(),
                    delegator_address: todo!(),
                    pubkey: todo!(),
                    value: None,
                },
                &signer,
            )
            .unwrap();
    }

    #[test]
    fn it_can_edit_a_validator() {
        let app = InjectiveTestApp::new();
        let signer = app
            .init_account(&[CosmCoin::new(100_000_000_000_000_000_000_000_000u128, INJ)])
            .unwrap();

        let validator_signing_account = app
            .get_first_validator_signing_account(INJ.to_string(), 0.12f64)
            .unwrap();
        let validator_address = app.get_first_validator_address().unwrap();

        let banker = app
            .init_account(&[CosmCoin::new(100_000_000_000_000_000_000_000_000u128, INJ)])
            .unwrap();

        fund_account_with_some_inj(
            &Bank::new(&app),
            &banker,
            &validator_signing_account.address(),
        );

        let staking = Staking::new(&app);
        staking
            .edit_validator(
                MsgEditValidator {
                    validator_address,
                    description: Some(Description {
                        moniker: "new moniker".to_string(),
                        identity: "new identity".to_string(),
                        website: "new website".to_string(),
                        security_contact: "new security contact".to_string(),
                        details: "new details".to_string(),
                    }),
                    commission_rate: "5000000".to_string(),
                    min_self_delegation: "2000".to_string(),
                },
                &signer,
            )
            .unwrap();
    }

    fn fund_account_with_some_inj(bank: &Bank<InjectiveTestApp>, from: &SigningAccount, to: &str) {
        bank.send(
            MsgSend {
                from_address: from.address(),
                to_address: to.to_string(),
                amount: vec![Coin {
                    amount: "100000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
            },
            from,
        )
        .unwrap();
    }
}
