fn main() {
    prost_build::Config::new()
        .out_dir("src/pb")
        .protoc_executable("./protoc.exe")
        .compile_protos(&["orwell.proto"], &["."])
        .unwrap();
}
