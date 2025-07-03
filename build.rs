fn main() {
    prost_build::Config::new()
        .out_dir("src/pb")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["orwell.proto"], &["."])
        .unwrap();
}
