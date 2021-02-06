use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};

use aead::{Aead, Error, NewAead, Nonce, Payload};
use bytes::BytesMut;
use cwc::Aes128Cwc;

#[derive(Clone)]
pub enum CipherMode {
    Rsa(Rsa<Private>, Padding),
    Cwc(Aes128Cwc),
}

impl CipherMode {
    pub fn rsa_x931(key: &[u8]) -> Self {
        CipherMode::Rsa(
            Rsa::private_key_from_pem(key).expect("invalid key"),
            Padding::from_raw(5),
        )
    }

    pub fn rsa_pkcs1_oeap(key: &[u8]) -> Self {
        CipherMode::Rsa(
            Rsa::private_key_from_pem(key).expect("invalid key"),
            Padding::PKCS1_OAEP,
        )
    }

    pub fn aes128_cwc(key: &[u8]) -> Self {
        CipherMode::Cwc(Aes128Cwc::new_varkey(key).expect("invalid key"))
    }
}

pub fn decrypt(mode: &CipherMode, input: &[u8]) -> Result<BytesMut, Error> {
    match mode {
        CipherMode::Rsa(key, padding) => {
            let mut decrypted_data = BytesMut::with_capacity(input.len());
            let decrypted_len = key
                .private_decrypt(input, &mut decrypted_data, *padding)
                .map_err(|_| Error)?;
            decrypted_data.truncate(decrypted_len);

            Ok(decrypted_data)
        }
        CipherMode::Cwc(key) => {
            let nonce = &input[0..11];
            let tag = &input[11..27];
            let data = &input[27..];

            let mut payload = Payload::from(data);
            payload.aad = tag;

            let decrypted_data = key.decrypt(Nonce::from_slice(nonce), payload)?;

            Ok(BytesMut::from(&decrypted_data[..]))
        }
    }
}

pub fn encrypt(mode: &CipherMode, input: &[u8]) -> Vec<u8> {
    unimplemented!()
}
