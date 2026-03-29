fn main() {
    prost_build::compile_protos(&["proto/messages.proto"], &["proto/"]).unwrap();
}
