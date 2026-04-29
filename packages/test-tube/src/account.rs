use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    AccountId,
};
use cosmwasm_std::Coin;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::PublicKey as K256PublicKey;
use sha3::{Digest, Keccak256};

pub trait Account {
    fn public_key(&self) -> PublicKey;
    fn derivation(&self) -> AddressDerivation;
    fn address(&self) -> String {
        self.account_id().to_string()
    }
    fn prefix(&self) -> &str;
    fn account_id(&self) -> AccountId {
        derive_account_id(&self.public_key(), self.prefix(), self.derivation())
            .expect("account derivation should be valid for supported secp256k1 accounts")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AddressDerivation {
    #[default]
    Cosmos = 0,
    InjectiveEvm = 1,
}

pub fn derive_account_id(
    public_key: &PublicKey,
    prefix: &str,
    derivation: AddressDerivation,
) -> cosmrs::Result<AccountId> {
    match derivation {
        AddressDerivation::Cosmos => public_key.account_id(prefix),
        AddressDerivation::InjectiveEvm => {
            let evm_address_bytes = derive_evm_address_bytes(public_key)?;
            AccountId::new(prefix, &evm_address_bytes)
        }
    }
}

pub fn derive_evm_address_bytes(public_key: &PublicKey) -> cosmrs::Result<[u8; 20]> {
    let encoded_point = K256PublicKey::from_sec1_bytes(&public_key.to_bytes())
        .map_err(|_| cosmrs::Error::Crypto)?
        .to_encoded_point(false);

    let uncompressed_bytes = encoded_point.as_bytes();
    let hash = Keccak256::digest(&uncompressed_bytes[1..]);

    Ok(hash[12..]
        .try_into()
        .expect("last 20 bytes of a keccak hash must fit into an address"))
}

pub struct SigningAccount {
    prefix: String,
    signing_key: SigningKey,
    private_key_bytes: [u8; 32],
    derivation: AddressDerivation,
    fee_setting: FeeSetting,
}

impl SigningAccount {
    pub fn new(
        prefix: String,
        signing_key: SigningKey,
        private_key_bytes: [u8; 32],
        fee_setting: FeeSetting,
    ) -> Self {
        Self::new_with_derivation(
            prefix,
            signing_key,
            private_key_bytes,
            AddressDerivation::Cosmos,
            fee_setting,
        )
    }

    pub fn new_with_derivation(
        prefix: String,
        signing_key: SigningKey,
        private_key_bytes: [u8; 32],
        derivation: AddressDerivation,
        fee_setting: FeeSetting,
    ) -> Self {
        SigningAccount {
            prefix,
            signing_key,
            private_key_bytes,
            derivation,
            fee_setting,
        }
    }

    pub fn with_prefix(self, prefix: String) -> Self {
        Self {
            prefix,
            signing_key: self.signing_key,
            private_key_bytes: self.private_key_bytes,
            derivation: self.derivation,
            fee_setting: self.fee_setting,
        }
    }

    pub fn fee_setting(&self) -> &FeeSetting {
        &self.fee_setting
    }

    pub fn derivation(&self) -> AddressDerivation {
        self.derivation
    }

    pub fn with_derivation(self, derivation: AddressDerivation) -> Self {
        Self {
            prefix: self.prefix,
            signing_key: self.signing_key,
            private_key_bytes: self.private_key_bytes,
            derivation,
            fee_setting: self.fee_setting,
        }
    }

    pub fn with_fee_setting(self, fee_setting: FeeSetting) -> Self {
        Self {
            prefix: self.prefix,
            signing_key: self.signing_key,
            private_key_bytes: self.private_key_bytes,
            derivation: self.derivation,
            fee_setting,
        }
    }
}

impl Account for SigningAccount {
    fn public_key(&self) -> PublicKey {
        self.signing_key.public_key()
    }

    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn derivation(&self) -> AddressDerivation {
        self.derivation
    }
}

impl SigningAccount {
    pub fn signing_key(&'_ self) -> &'_ SigningKey {
        &self.signing_key
    }

    pub fn private_key_bytes(&self) -> &[u8; 32] {
        &self.private_key_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonSigningAccount {
    prefix: String,
    public_key: PublicKey,
    derivation: AddressDerivation,
}

impl From<PublicKey> for NonSigningAccount {
    fn from(public_key: PublicKey) -> Self {
        NonSigningAccount {
            prefix: String::from(""),
            public_key,
            derivation: AddressDerivation::Cosmos,
        }
    }
}
impl From<SigningAccount> for NonSigningAccount {
    fn from(signing_account: SigningAccount) -> Self {
        NonSigningAccount {
            prefix: signing_account.prefix.clone(),
            public_key: signing_account.public_key(),
            derivation: signing_account.derivation,
        }
    }
}

impl NonSigningAccount {
    pub fn new(prefix: String, public_key: PublicKey) -> Self {
        Self::new_with_derivation(prefix, public_key, AddressDerivation::Cosmos)
    }

    pub fn new_with_derivation(
        prefix: String,
        public_key: PublicKey,
        derivation: AddressDerivation,
    ) -> Self {
        NonSigningAccount {
            prefix,
            public_key,
            derivation,
        }
    }

    pub fn with_prefix(self, prefix: String) -> Self {
        Self {
            prefix,
            public_key: self.public_key,
            derivation: self.derivation,
        }
    }

    pub fn derivation(&self) -> AddressDerivation {
        self.derivation
    }

    pub fn with_derivation(self, derivation: AddressDerivation) -> Self {
        Self {
            prefix: self.prefix,
            public_key: self.public_key,
            derivation,
        }
    }
}

impl Account for NonSigningAccount {
    fn public_key(&self) -> PublicKey {
        self.public_key
    }

    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn derivation(&self) -> AddressDerivation {
        self.derivation
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeeSetting {
    Auto {
        gas_price: Coin,
        gas_adjustment: f64,
    },
    Custom {
        amount: Coin,
        gas_limit: u64,
    },
}

#[cfg(test)]
mod tests {
    use cosmrs::crypto::secp256k1::SigningKey;

    use super::{derive_evm_address_bytes, Account, AddressDerivation, FeeSetting, SigningAccount};

    #[test]
    fn injective_evm_derivation_changes_account_address() {
        let signing_key =
            SigningKey::from_slice(&[7u8; 32]).expect("test private key should be valid");
        let private_key_bytes = [7u8; 32];
        let fee_setting = FeeSetting::Custom {
            amount: cosmwasm_std::Coin::new(1u128, "inj"),
            gas_limit: 1,
        };

        let cosmos_account = SigningAccount::new(
            "inj".to_string(),
            signing_key,
            private_key_bytes,
            fee_setting.clone(),
        );
        let cosmos_address = cosmos_account.address();
        let injective_evm_account = cosmos_account.with_derivation(AddressDerivation::InjectiveEvm);

        assert_ne!(cosmos_address, injective_evm_account.address());
        assert_eq!(
            injective_evm_account.account_id().to_bytes(),
            derive_evm_address_bytes(&injective_evm_account.public_key())
                .expect("test key should derive an EVM address")
        );
    }
}
