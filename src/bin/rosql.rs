//! `rosql` CLI — parse, compile, and execute ROSQL queries.
//!
//! Build with: `cargo build --features server --bin rosql`
//! For query execution: `cargo build --features server,postgres --bin rosql`
//!
//! Usage:
//!   rosql parse <query>                          # parse → JSON AST
//!   rosql compile <query> --backend <type>       # parse → compiled SQL
//!   rosql query <query> --backend <type> --url   # parse → execute → results
//!   rosql validate <query>                       # validate syntax
//!   rosql completions <query> <pos>              # autocomplete
//!   rosql serve [--socket <path>]                # gRPC server

use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, Read};

#[derive(Parser)]
#[command(
    name = "rosql",
    about = "ROSQL — parse, compile, and execute ROS2 telemetry queries"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a ROSQL query and output the AST as JSON.
    Parse {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,
    },

    /// Compile a ROSQL query to SQL for a specific backend.
    Compile {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Target database backend.
        #[arg(long, default_value = "postgres")]
        backend: Backend,

        /// Schema profile — determines column naming convention.
        /// Defaults to otel-postgres for postgres/mysql/sqlite backends.
        #[arg(long, default_value = "otel-postgres")]
        schema: Schema,
    },

    /// Execute a ROSQL query against a database and return results.
    Query {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Target database backend.
        #[arg(long)]
        backend: Backend,

        /// Schema profile — determines column naming convention.
        #[arg(long, default_value = "otel-postgres")]
        schema: Schema,

        /// Database connection URL.
        /// PostgreSQL: postgresql://user:pass@host:5432/db
        /// SQLite: sqlite:./path/to/db
        /// MySQL: mysql://user:pass@host:3306/db
        #[arg(long)]
        url: String,
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

    /// Start the gRPC parser server.
    Serve {
        /// Unix socket path for the gRPC server.
        #[arg(long, default_value = "/tmp/rosql.sock")]
        socket: String,

        /// Log level.
        #[arg(long, default_value = "info")]
        log_level: String,
    },
}

/// Schema profile — column naming convention for the OTel exporter.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Schema {
    /// Lowercase columns (OTel Collector PostgreSQL exporter convention).
    /// Example: trace_id, status_code, span_attributes
    #[value(name = "otel-postgres")]
    OtelPostgres,
    /// PascalCase columns (OTel Collector ClickHouse exporter convention).
    /// Example: TraceId, StatusCode, SpanAttributes
    #[value(name = "otel-clickhouse")]
    OtelClickhouse,
}

impl Schema {
    fn to_profile(self) -> rosql::drivers::otel_registry::SchemaProfile {
        match self {
            Schema::OtelPostgres => rosql::drivers::otel_registry::SchemaProfile::OtelPostgres,
            Schema::OtelClickhouse => rosql::drivers::otel_registry::SchemaProfile::OtelClickhouse,
        }
    }
}

/// Supported database backends.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Backend {
    /// PostgreSQL / TimescaleDB
    Postgres,
    /// MySQL / MariaDB
    Mysql,
    /// Parquet files (local path or s3://) powered by DuckDB.
    /// Use --url to point at a directory containing traces/, logs/, etc.
    Parquet,
    /// AWS Athena (coming soon)
    Athena,
    /// Google BigQuery (coming soon)
    Bigquery,
}

impl Backend {
    fn to_dialect(self) -> Result<rosql::drivers::dialect::SqlDialect, String> {
        match self {
            Backend::Postgres => Ok(rosql::drivers::dialect::SqlDialect::PostgreSQL),
            Backend::Mysql => Ok(rosql::drivers::dialect::SqlDialect::MySQL),
            Backend::Parquet => Ok(rosql::drivers::dialect::SqlDialect::DuckDB),
            Backend::Athena => Err("Athena backend is not yet supported. See https://github.com/RobotOpsInc/rosql/issues/9".into()),
            Backend::Bigquery => Err("BigQuery backend is not yet supported. See https://github.com/RobotOpsInc/rosql/issues/10".into()),
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { query } => {
            let query = read_query(query);
            cmd_parse(&query);
        }
        Commands::Compile {
            query,
            backend,
            schema,
        } => {
            let query = read_query(query);
            cmd_compile(&query, backend, schema);
        }
        Commands::Query {
            query,
            backend,
            schema,
            url,
        } => {
            let query = read_query(query);
            cmd_query(&query, backend, schema, &url).await;
        }
        Commands::Validate { query } => {
            let query = read_query(query);
            cmd_validate(&query);
        }
        Commands::Completions { query, cursor_pos } => {
            cmd_completions(&query, cursor_pos);
        }
        Commands::Serve { socket, log_level } => {
            serve(socket, log_level).await;
        }
    }
}

// ---------------------------------------------------------------------------
// CLI commands
// ---------------------------------------------------------------------------

fn cmd_parse(query: &str) {
    match rosql::parse(query) {
        Ok(ast) => {
            let json = serde_json::json!({
                "ok": true,
                "ast": serde_json::to_value(&ast).unwrap_or(serde_json::Value::Null),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(errors) => {
            print_errors(&errors);
            std::process::exit(1);
        }
    }
}

fn cmd_compile(query: &str, backend: Backend, schema: Schema) {
    let dialect = match backend.to_dialect() {
        Ok(d) => d,
        Err(msg) => {
            eprintln!("Error: {msg}");
            std::process::exit(1);
        }
    };

    let ast = match rosql::parse(query) {
        Ok(ast) => ast,
        Err(errors) => {
            print_errors(&errors);
            std::process::exit(1);
        }
    };

    let registry = rosql::drivers::otel_registry::otel_registry(schema.to_profile());
    let capabilities = rosql::BackendCapabilities {
        topic_data: true,
        recording_index: true,
    };

    match rosql::drivers::compiler::compile(&ast, &registry, &dialect, &capabilities, None) {
        Ok(cr) => {
            let json = serde_json::json!({
                "ok": true,
                "sql": cr.sql,
                "backend": format!("{backend:?}").to_lowercase(),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(err) => {
            let json = serde_json::json!({
                "ok": false,
                "error": err.to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            std::process::exit(1);
        }
    }
}

#[allow(unused_variables)]
async fn cmd_query(query: &str, backend: Backend, _schema: Schema, url: &str) {
    #[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
    {
        use rosql::drivers::ROSQLBackend;

        // Validate backend is supported (errors out on Athena/BigQuery).
        if let Err(msg) = backend.to_dialect() {
            eprintln!("Error: {msg}");
            std::process::exit(1);
        }

        let ast = match rosql::parse(query) {
            Ok(ast) => ast,
            Err(errors) => {
                print_errors(&errors);
                std::process::exit(1);
            }
        };

        let sql_backend = match backend {
            #[cfg(feature = "duckdb")]
            Backend::Parquet => {
                match rosql::drivers::sql::SqlBackend::from_parquet(url).await {
                    Ok(b) => b,
                    Err(err) => {
                        eprintln!("Parquet backend error: {err}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                match rosql::drivers::sql::SqlBackend::new(url).await {
                    Ok(b) => b,
                    Err(err) => {
                        eprintln!("Connection error: {err}");
                        std::process::exit(1);
                    }
                }
            }
        };

        let opts = rosql::ExecOptions::default();
        match sql_backend.execute(&ast, &opts).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result).unwrap();
                println!("{json}");
            }
            Err(err) => {
                let json = serde_json::json!({
                    "ok": false,
                    "error": err.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(any(feature = "postgres", feature = "mysql", feature = "duckdb")))]
    {
        eprintln!(
            "Error: the `query` subcommand requires a database driver feature.\n\
             Rebuild with one of:\n\
             cargo build --features server,postgres --bin rosql\n\
             cargo build --features server,mysql --bin rosql\n\
             cargo build --features server,duckdb --bin rosql  # for --backend parquet"
        );
        std::process::exit(1);
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

fn print_errors(errors: &[rosql::ROSQLError]) {
    let error_list: Vec<serde_json::Value> = errors
        .iter()
        .map(|e| serde_json::json!({ "error": e.to_string() }))
        .collect();
    let json = serde_json::json!({
        "ok": false,
        "errors": error_list,
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
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

    eprintln!("rosql-parser gRPC server starting on {socket}");
    let _ = std::fs::remove_file(&socket);

    let uds = tokio::net::UnixListener::bind(&socket).expect("failed to bind Unix socket");
    let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

    tonic::transport::Server::builder()
        .add_service(RosqlParserServiceServer::new(ParserService))
        .serve_with_incoming(uds_stream)
        .await
        .expect("gRPC server failed");
}
