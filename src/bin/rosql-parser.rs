//! `rosql-parser` — gRPC server + CLI for ROSQL parsing.
//!
//! Build with: `cargo build --features server --bin rosql-parser`
//!
//! Usage:
//!   rosql-parser serve [--socket <path>]     # gRPC server mode
//!   rosql-parser parse <query>               # CLI parse to JSON
//!   rosql-parser validate <query>            # CLI validate
//!   rosql-parser completions <query> <pos>   # CLI completions

use clap::{Parser, Subcommand};
use std::io::{self, Read};

#[derive(Parser)]
#[command(name = "rosql-parser", about = "ROSQL parser — gRPC server and CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gRPC parser server.
    Serve {
        /// Unix socket path for the gRPC server.
        #[arg(long, default_value = "/tmp/rosql-parser.sock")]
        socket: String,

        /// Log level.
        #[arg(long, default_value = "info")]
        log_level: String,
    },

    /// Parse a ROSQL query and output the AST as JSON.
    Parse {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,
    },

    /// Validate a ROSQL query.
    Validate {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,
    },

    /// Get completions at a cursor position.
    Completions {
        /// The ROSQL query string.
        query: String,
        /// Cursor position (0-based byte offset).
        cursor_pos: usize,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { socket, log_level } => {
            serve(socket, log_level).await;
        }
        Commands::Parse { query } => {
            let query = read_query(query);
            cmd_parse(&query);
        }
        Commands::Validate { query } => {
            let query = read_query(query);
            cmd_validate(&query);
        }
        Commands::Completions { query, cursor_pos } => {
            cmd_completions(&query, cursor_pos);
        }
    }
}

// ---------------------------------------------------------------------------
// CLI commands
// ---------------------------------------------------------------------------

fn cmd_parse(query: &str) {
    match rosql::parse(query) {
        Ok(ast) => {
            let _proto = rosql::convert::query_to_proto(&ast);
            let json = serde_json::json!({
                "ok": true,
                "ast": serde_json::to_value(&ast).unwrap_or(serde_json::Value::Null),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(errors) => {
            let error_list: Vec<serde_json::Value> = errors
                .iter()
                .map(|e| serde_json::json!({ "error": e.to_string() }))
                .collect();
            let json = serde_json::json!({
                "ok": false,
                "errors": error_list,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            std::process::exit(1);
        }
    }
}

fn cmd_validate(query: &str) {
    match rosql::parse(query) {
        Ok(_) => {
            let json = serde_json::json!({
                "valid": true,
                "errors": [],
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(errors) => {
            let error_list: Vec<serde_json::Value> = errors
                .iter()
                .map(|e| serde_json::json!({ "error": e.to_string() }))
                .collect();
            let json = serde_json::json!({
                "valid": false,
                "errors": error_list,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            std::process::exit(1);
        }
    }
}

fn cmd_completions(query: &str, cursor_pos: usize) {
    let completions = rosql::completions::get_completions(query, cursor_pos);
    let json = serde_json::to_string_pretty(&completions).unwrap();
    println!("{json}");
}

fn read_query(query: Option<String>) -> String {
    match query {
        Some(q) => q,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .expect("failed to read from stdin");
            buf.trim().to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// gRPC server
// ---------------------------------------------------------------------------

async fn serve(socket: String, _log_level: String) {
    use rosql::proto::rosql_v1::rosql_parser_service_server::{
        RosqlParserService, RosqlParserServiceServer,
    };
    use rosql::proto::rosql_v1::*;
    use tonic::{Request, Response, Status};

    struct ParserService;

    #[tonic::async_trait]
    impl RosqlParserService for ParserService {
        async fn parse(
            &self,
            request: Request<ParseRequest>,
        ) -> Result<Response<ParseResponse>, Status> {
            let req = request.into_inner();
            match rosql::parse(&req.query) {
                Ok(ast) => {
                    let proto_query = rosql::convert::query_to_proto(&ast);
                    Ok(Response::new(ParseResponse {
                        result: Some(parse_response::Result::Query(proto_query)),
                    }))
                }
                Err(errors) => {
                    let proto_err = rosql::convert::error_to_proto(&errors[0]);
                    Ok(Response::new(ParseResponse {
                        result: Some(parse_response::Result::Error(proto_err)),
                    }))
                }
            }
        }

        async fn validate(
            &self,
            request: Request<ValidateRequest>,
        ) -> Result<Response<ValidateResponse>, Status> {
            let req = request.into_inner();
            match rosql::parse(&req.query) {
                Ok(_) => Ok(Response::new(ValidateResponse {
                    valid: true,
                    errors: vec![],
                    warnings: vec![],
                })),
                Err(errors) => {
                    let diagnostics: Vec<ValidationDiagnostic> = errors
                        .iter()
                        .map(|e| {
                            let (message, location, suggestion) = match e {
                                rosql::ROSQLError::ParseError {
                                    message,
                                    location,
                                    suggestion,
                                } => (
                                    message.clone(),
                                    Some(rosql::convert::source_location_to_proto(location)),
                                    suggestion.clone().unwrap_or_default(),
                                ),
                                other => (other.to_string(), None, String::new()),
                            };
                            ValidationDiagnostic {
                                message,
                                location,
                                severity: DiagnosticSeverity::Error as i32,
                                suggestion,
                            }
                        })
                        .collect();
                    Ok(Response::new(ValidateResponse {
                        valid: false,
                        errors: diagnostics,
                        warnings: vec![],
                    }))
                }
            }
        }

        async fn get_completions(
            &self,
            request: Request<GetCompletionsRequest>,
        ) -> Result<Response<GetCompletionsResponse>, Status> {
            let req = request.into_inner();
            let completions =
                rosql::completions::get_completions(&req.query, req.cursor_position as usize);

            let proto_completions: Vec<Completion> = completions
                .into_iter()
                .map(|c| {
                    let kind = match c.kind {
                        rosql::completions::CompletionKind::Keyword => CompletionKind::Keyword,
                        rosql::completions::CompletionKind::DataSource => {
                            CompletionKind::DataSource
                        }
                        rosql::completions::CompletionKind::Field => CompletionKind::Field,
                        rosql::completions::CompletionKind::Function => CompletionKind::Function,
                        rosql::completions::CompletionKind::Unit => CompletionKind::Unit,
                    };
                    Completion {
                        label: c.label,
                        detail: c.detail,
                        kind: kind as i32,
                    }
                })
                .collect();

            Ok(Response::new(GetCompletionsResponse {
                completions: proto_completions,
            }))
        }
    }

    // Listen on Unix socket
    eprintln!("rosql-parser gRPC server starting on {socket}");

    // Remove stale socket file if it exists
    let _ = std::fs::remove_file(&socket);

    let uds = tokio::net::UnixListener::bind(&socket).expect("failed to bind Unix socket");
    let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

    tonic::transport::Server::builder()
        .add_service(RosqlParserServiceServer::new(ParserService))
        .serve_with_incoming(uds_stream)
        .await
        .expect("gRPC server failed");
}
