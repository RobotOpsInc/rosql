use std::io::Result;

fn main() -> Result<()> {
    // Generate Rust types from proto files into src/proto/
    // The generated files are checked into the repo.
    let out_dir = "src/proto";
    std::fs::create_dir_all(out_dir)?;

    prost_build::Config::new().out_dir(out_dir).compile_protos(
        &[
            "proto/rosql/v1/parser_service.proto",
            "proto/rosql/v1/ast.proto",
            "proto/rosql/v1/result.proto",
            "proto/rosql/v1/field_registry.proto",
        ],
        &["proto/"],
    )?;

    Ok(())
}
