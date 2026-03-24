use std::io::Result;

fn main() -> Result<()> {
    let out_dir = "src/proto";
    std::fs::create_dir_all(out_dir)?;

    // Rerun if proto files change or if features change
    println!("cargo:rerun-if-changed=proto/");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERVER");

    let protos = &[
        "proto/rosql/v1/parser_service.proto",
        "proto/rosql/v1/ast.proto",
        "proto/rosql/v1/result.proto",
        "proto/rosql/v1/field_registry.proto",
    ];
    let includes = &["proto/"];

    // When the `server` feature is enabled, generate gRPC service traits
    // via tonic-build (includes prost message types automatically).
    // Otherwise, generate only prost message types.
    if std::env::var("CARGO_FEATURE_SERVER").is_ok() {
        tonic_build::configure()
            .out_dir(out_dir)
            .compile_protos(protos, includes)?;
    } else {
        prost_build::Config::new()
            .out_dir(out_dir)
            .compile_protos(protos, includes)?;
    }

    Ok(())
}
