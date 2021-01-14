extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/protobuf/Frpg2RequestMessage.proto"],
                                &["src/protobuf/"]).unwrap();
}