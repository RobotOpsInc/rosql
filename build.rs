use std::io::Result;
fn main() -> Result<()> {
    let protos = &[
        "proto/rosql/v1/parser_service.proto",
        "proto/rosql/v1/ast.proto",
        "proto/rosql/v1/result.proto",
        "proto/rosql/v1/field_registry.proto",
    ];
    let includes = &["proto/"];

    if std::env::var("CARGO_FEATURE_SERVER").is_ok() {
        // With server feature: use tonic-build which generates both
        // message types AND gRPC service traits in one file.
        tonic_build::configure()
            .build_server(true)
            .build_client(false)
            .compile_protos(protos, includes)?;
    } else {
        // Without server feature: use prost-build for message types only.
        // No tonic dependency in the generated code.
        prost_build::compile_protos(protos, includes)?;
    }

    // Ensure rebuild when features change.
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERVER");
    for proto in protos {
        println!("cargo:rerun-if-changed={proto}");
    }

    Ok(())
}
