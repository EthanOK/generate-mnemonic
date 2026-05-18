//! ETH address via Alloy [`MnemonicBuilder`], aligned with `cast wallet address` etc.

use alloy::signers::local::{
    coins_bip39::{English, Mnemonic},
    LocalSignerError, MnemonicBuilder,
};

pub fn address_from_mnemonic_at_path(
    m: &Mnemonic<English>,
    derivation_path: &str,
) -> Result<String, LocalSignerError> {
    let signer = MnemonicBuilder::<English>::default()
        .phrase(m.to_phrase())
        .derivation_path(derivation_path)?
        .build()?;
    Ok(signer.address().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::local::coins_bip39::{English, Mnemonic};

    #[test]
    fn abandon_mnemonic_first_eth_matches_cast() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let m: Mnemonic<English> = phrase.parse().unwrap();
        let addr = address_from_mnemonic_at_path(&m, crate::chain::ETH_DEFAULT_PATH).unwrap();
        assert_eq!(
            addr.to_lowercase(),
            "0x9858effd232b4033e47d90003d41ec34ecaeda94"
        );
        assert_ne!(
            addr,
            addr.to_lowercase(),
            "EIP-55 output should be mixed-case for vanity --strict"
        );
    }
}
