extern crate prost_build;

fn main() {
    println!("cargo:rerun-if-changed=proto/");

    prost_build::compile_protos(
        &["proto/frpg2_request.proto", "proto/common.proto"],
        &["proto/"],
    )
    .unwrap();
}
