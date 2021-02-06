extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/msg/frpg2_request.proto", "src/msg/common.proto"],
                                &["src/msg/"]).unwrap();
}