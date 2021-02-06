use openssl::pkey::Private;
use openssl::rsa::Rsa;

fn main() {
    let key = Rsa::generate(2048).expect("couldn't generate key");
    let private_pem = key.private_key_to_pem().expect("unable to encode key");
    let public_pem = key
        .public_key_to_pem_pkcs1()
        .expect("unable to encode public key");

    println!(
        "private key = {}",
        String::from_utf8(private_pem).expect("invalid string")
    );
    println!(
        "public key = {}",
        String::from_utf8(public_pem).expect("invalid string")
    );
}
