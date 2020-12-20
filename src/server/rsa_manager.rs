use std::sync::Arc;

use parking_lot::RwLock;

extern crate openssl;
use openssl::rsa::*;
use openssl::pkey::Private;

use crate::Config;

pub struct RsaManager {
    rsa_priv_key_inst: Rsa<Private> 
}

impl RsaManager {
    pub fn new(config_inst: Arc<RwLock<Config>>) -> RsaManager {
        let config_read_guard = config_inst.read();
        let rsa_private_key_str = config_read_guard.get_rsa_private_key();
        let rsa_priv_key_inst = Rsa::private_key_from_pem(rsa_private_key_str.as_bytes()).unwrap();

        RsaManager {
            rsa_priv_key_inst
        } 
    }

    pub fn rsa_encrypt(&self, from: &[u8], to: &mut [u8]) -> usize {
        // RSA_X931_PADDING is used by the server, it isn't exposed in the rust openssl so we use a raw value to access it
        // I also can't be bothered to make a pull request or a fork
        let length = self.rsa_priv_key_inst.private_encrypt( from, to, Padding::from_raw(5)).unwrap(); 
        return length;
    }

    pub fn rsa_decrypt(&self, from: &[u8], to: &mut [u8]) -> usize {
        let length = self.rsa_priv_key_inst.private_decrypt( from, to, Padding::PKCS1_OAEP).unwrap();
        return length;
    }
}