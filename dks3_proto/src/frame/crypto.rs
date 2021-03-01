use std::fmt::{Debug, Formatter};

use aead::{Aead, AeadInPlace, Error, NewAead, Nonce, Payload, Tag};
use block_cipher::{Block, BlockCipher, NewBlockCipher};

use bytes::BytesMut;
use cwc::Aes128Cwc;
use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};
use rand::Rng;
use tracing::info;

#[derive(Clone)]
pub enum CipherMode {
    Rsa(Rsa<Private>, Padding),
    Cwc(Aes128Cwc),
}

impl Debug for CipherMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CipherMode::Rsa(_, _) => write!(f, "Rsa")?,
            CipherMode::Cwc(_) => write!(f, "Cwc")?,
        };

        Ok(())
    }
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
            let mut decrypted_data = BytesMut::with_capacity(key.size() as usize);
            decrypted_data.resize(key.size() as usize, 0);

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

            let mut plaintext = Vec::from(data);
            key.decrypt_in_place_detached(
                Nonce::from_slice(nonce),
                nonce,
                &mut plaintext,
                Tag::from_slice(tag),
            )?;

            Ok(BytesMut::from(&plaintext[..]))
        }
    }
}

pub fn encrypt(mode: &CipherMode, input: &[u8]) -> Result<BytesMut, Error> {
    match mode {
        CipherMode::Rsa(key, padding) => {
            let mut encrypted_data = BytesMut::with_capacity(key.size() as usize);
            encrypted_data.resize(key.size() as usize, 0);

            let encrypted_len = key
                .private_encrypt(input, &mut encrypted_data, *padding)
                .expect("couldn't encrypt");
            encrypted_data.truncate(encrypted_len);

            Ok(encrypted_data)
        }
        CipherMode::Cwc(key) => {
            let iv = Nonce::from(rand::thread_rng().gen::<[u8; 11]>());
            let mut data = Vec::with_capacity(11 + 16 + input.len());
            unsafe { data.set_len(11 + 16 + input.len()) };
            data[0..11].copy_from_slice(iv.as_slice());
            data[11 + 16..].copy_from_slice(input);

            let tag = key.encrypt_in_place_detached(&iv, iv.as_slice(), &mut data[11 + 16..])?;
            data[11..11 + 16].copy_from_slice(tag.as_slice());

            Ok(BytesMut::from(&data[..]))
        }
    }
}
