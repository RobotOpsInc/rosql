//! `rosql` CLI — parse, compile, and execute ROSQL queries.
//!
//! Build with: `cargo build --features server --bin rosql`
//! For query execution: `cargo build --features server,postgres --bin rosql`
//!                      `cargo build --features server,duckdb --bin rosql`
//!
//! Usage:
//!   rosql parse <query>                          # parse → JSON AST
//!   rosql compile <query> --backend <type>       # parse → compiled SQL
//!   rosql query <query> --backend <type> --url   # parse → execute → results
//!   rosql validate <query>                       # validate syntax
//!   rosql schema --backend <type> --url          # inspect available data sources
//!   rosql completions <query> <pos>              # autocomplete
//!   rosql serve [--socket <path>]                # gRPC server

use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, IsTerminal, Read};

// ---------------------------------------------------------------------------
// Config file — ~/.config/rosql/config.toml
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
struct Config {
    default: Option<ConfigDefaults>,
}

#[derive(Debug, serde::Deserialize)]
struct ConfigDefaults {
    backend: Option<String>,
    url: Option<String>,
    schema: Option<String>,
}

fn load_config() -> Config {
    let Some(config_dir) = dirs::config_dir() else {
        return Config::default();
    };
    let path = config_dir.join("rosql").join("config.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    toml::from_str(&content).unwrap_or_default()
}

fn config_backend(config: &Config) -> Option<Backend> {
    let s = config.default.as_ref()?.backend.as_deref()?;
    match s.to_lowercase().as_str() {
        "postgres" | "postgresql" => Some(Backend::Postgres),
        "mysql" | "mariadb" => Some(Backend::Mysql),
        "parquet" | "duckdb" => Some(Backend::Parquet),
        _ => None,
    }
}

fn config_schema(config: &Config) -> Option<Schema> {
    let s = config.default.as_ref()?.schema.as_deref()?;
    match s.to_lowercase().as_str() {
        "otel-postgres" | "otel_postgres" => Some(Schema::OtelPostgres),
        "otel-clickhouse" | "otel_clickhouse" => Some(Schema::OtelClickhouse),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "rosql",
    about = "ROSQL — parse, compile, and execute ROS2 telemetry queries",
    version
)]
struct Cli {
    /// Disable ANSI color codes in all output.
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a ROSQL query and output the AST as JSON.
    Parse {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Read the query from a file instead of a positional argument.
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },

    /// Compile a ROSQL query to SQL for a specific backend.
    Compile {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Read the query from a file instead of a positional argument.
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Target database backend.
        #[arg(long, default_value = "postgres", env = "ROSQL_BACKEND")]
        backend: Backend,

        /// Schema profile — determines column naming convention.
        #[arg(long, default_value = "otel-postgres", env = "ROSQL_SCHEMA")]
        schema: Schema,
    },

    /// Execute a ROSQL query against a database and return results.
    Query {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Read the query from a file instead of a positional argument.
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Target database backend.
        #[arg(long, env = "ROSQL_BACKEND")]
        backend: Option<Backend>,

        /// Schema profile — determines column naming convention.
        #[arg(long, env = "ROSQL_SCHEMA")]
        schema: Option<Schema>,

        /// Database connection URL.
        /// PostgreSQL: postgresql://user:pass@host:5432/db
        /// Parquet:    /path/to/telemetry/  or  s3://bucket/prefix/
        /// MySQL:      mysql://user:pass@host:3306/db
        #[arg(long, env = "ROSQL_URL")]
        url: Option<String>,

        /// Output format.
        #[arg(long, default_value = "table")]
        format: CliFormat,
    },

    /// Validate a ROSQL query.
    Validate {
        /// The ROSQL query string. Reads from stdin if omitted.
        query: Option<String>,

        /// Read the query from a file instead of a positional argument.
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },

    /// Inspect available data sources on the connected backend.
    SchemaCmd {
        /// Target database backend.
        #[arg(long, env = "ROSQL_BACKEND")]
        backend: Option<Backend>,

        /// Database connection URL.
        #[arg(long, env = "ROSQL_URL")]
        url: Option<String>,

        /// Output format.
        #[arg(long, default_value = "table")]
        format: CliFormat,
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

/// Output format for `query` and `schema` subcommands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliFormat {
    /// Human-readable aligned table (default).
    Table,
    /// JSON output for programmatic consumption.
    Json,
    /// CSV output.
    Csv,
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

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let no_color = cli.no_color || !io::stdout().is_terminal();
    let config = load_config();

    match cli.command {
        Commands::Parse { query, file } => {
            let query = read_query(query, file);
            cmd_parse(&query);
        }
        Commands::Compile {
            query,
            file,
            backend,
            schema,
        } => {
            let query = read_query(query, file);
            cmd_compile(&query, backend, schema);
        }
        Commands::Query {
            query,
            file,
            backend,
            schema,
            url,
            format,
        } => {
            let query = read_query(query, file);
            let backend = resolve_backend(backend, &config);
            let url = resolve_url(url, &config);
            let schema = resolve_schema(schema, &config);
            cmd_query(&query, backend, schema, &url, format, no_color).await;
        }
        Commands::Validate { query, file } => {
            let query = read_query(query, file);
            cmd_validate(&query);
        }
        Commands::SchemaCmd {
            backend,
            url,
            format,
        } => {
            let backend = resolve_backend(backend, &config);
            let url = resolve_url(url, &config);
            cmd_schema(backend, &url, format, no_color).await;
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
// Config resolution helpers
// ---------------------------------------------------------------------------

fn resolve_backend(cli_backend: Option<Backend>, config: &Config) -> Backend {
    cli_backend
        .or_else(|| config_backend(config))
        .unwrap_or_else(|| {
            eprintln!(
                "Error: --backend is required (or set ROSQL_BACKEND, or add to ~/.config/rosql/config.toml)"
            );
            std::process::exit(1);
        })
}

fn resolve_url(cli_url: Option<String>, config: &Config) -> String {
    cli_url
        .or_else(|| config.default.as_ref().and_then(|d| d.url.clone()))
        .unwrap_or_else(|| {
            eprintln!(
                "Error: --url is required (or set ROSQL_URL, or add to ~/.config/rosql/config.toml)"
            );
            std::process::exit(1);
        })
}

fn resolve_schema(cli_schema: Option<Schema>, config: &Config) -> Schema {
    cli_schema
        .or_else(|| config_schema(config))
        .unwrap_or(Schema::OtelPostgres)
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
async fn cmd_query(
    query: &str,
    backend: Backend,
    schema: Schema,
    url: &str,
    format: CliFormat,
    no_color: bool,
) {
    #[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
    {
        use rosql::drivers::ROSQLBackend;

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
            Backend::Parquet => match rosql::drivers::sql::SqlBackend::from_parquet(url).await {
                Ok(b) => b,
                Err(err) => {
                    eprintln!("Parquet backend error: {err}");
                    std::process::exit(1);
                }
            },
            _ => match rosql::drivers::sql::SqlBackend::new(url).await {
                Ok(b) => b,
                Err(err) => {
                    eprintln!("Connection error: {err}");
                    std::process::exit(1);
                }
            },
        };

        let opts = rosql::ExecOptions::default();
        match sql_backend.execute(&ast, &opts).await {
            Ok(result) => {
                print_result(&result, format, no_color);
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

#[allow(unused_variables)]
async fn cmd_schema(backend: Backend, url: &str, format: CliFormat, no_color: bool) {
    #[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
    {
        if let Err(msg) = backend.to_dialect() {
            eprintln!("Error: {msg}");
            std::process::exit(1);
        }

        let sql_backend = match backend {
            #[cfg(feature = "duckdb")]
            Backend::Parquet => match rosql::drivers::sql::SqlBackend::from_parquet(url).await {
                Ok(b) => b,
                Err(err) => {
                    eprintln!("Parquet backend error: {err}");
                    std::process::exit(1);
                }
            },
            _ => match rosql::drivers::sql::SqlBackend::new(url).await {
                Ok(b) => b,
                Err(err) => {
                    eprintln!("Connection error: {err}");
                    std::process::exit(1);
                }
            },
        };

        use rosql::drivers::ROSQLBackend;

        // Probe each canonical data source via a LIMIT 0 ROSQL query.
        let sources = [
            ("traces", "otel_traces"),
            ("logs", "otel_logs"),
            ("metrics", "otel_metrics"),
            ("topics", "topic_messages"),
            ("recordings", "mcap_metadata"),
        ];

        struct SourceRow {
            rosql_source: &'static str,
            table: &'static str,
            status: &'static str,
        }

        let mut rows: Vec<SourceRow> = Vec::new();
        for (rosql_source, table) in sources {
            let probe = format!("FROM {rosql_source} LIMIT 0");
            let status = match rosql::parse(&probe) {
                Ok(ast) => {
                    let opts = rosql::ExecOptions::default();
                    if sql_backend.execute(&ast, &opts).await.is_ok() {
                        "available"
                    } else {
                        "not found"
                    }
                }
                Err(_) => "not found",
            };
            rows.push(SourceRow {
                rosql_source,
                table,
                status,
            });
        }

        match format {
            CliFormat::Json => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "source": r.rosql_source,
                            "table": r.table,
                            "status": r.status,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json_rows).unwrap());
            }
            CliFormat::Csv => {
                println!("source,table,status");
                for r in &rows {
                    println!("{},{},{}", r.rosql_source, r.table, r.status);
                }
            }
            CliFormat::Table => {
                use colored::Colorize;
                use tabled::{Table, Tabled};

                #[derive(Tabled)]
                struct Row {
                    #[tabled(rename = "Source")]
                    source: String,
                    #[tabled(rename = "Table")]
                    table: String,
                    #[tabled(rename = "Status")]
                    status: String,
                }

                let table_rows: Vec<Row> = rows
                    .iter()
                    .map(|r| Row {
                        source: r.rosql_source.to_string(),
                        table: r.table.to_string(),
                        status: if no_color {
                            r.status.to_string()
                        } else if r.status == "available" {
                            r.status.green().to_string()
                        } else {
                            r.status.dimmed().to_string()
                        },
                    })
                    .collect();

                let table = Table::new(table_rows);
                println!("{table}");
            }
        }
    }

    #[cfg(not(any(feature = "postgres", feature = "mysql", feature = "duckdb")))]
    {
        eprintln!(
            "Error: the `schema` subcommand requires a database driver feature.\n\
             Rebuild with one of:\n\
             cargo build --features server,postgres --bin rosql\n\
             cargo build --features server,duckdb --bin rosql"
        );
        std::process::exit(1);
    }
}

fn cmd_completions(query: &str, cursor_pos: usize) {
    let completions = rosql::completions::get_completions(query, cursor_pos);
    let json = serde_json::to_string_pretty(&completions).unwrap();
    println!("{json}");
}

// ---------------------------------------------------------------------------
// Output rendering
// ---------------------------------------------------------------------------

#[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
fn print_result(result: &rosql::drivers::ROSQLResult, format: CliFormat, no_color: bool) {
    match format {
        CliFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result).unwrap());
        }
        CliFormat::Csv => {
            print_csv(result);
        }
        CliFormat::Table => {
            print_table(result, no_color);
        }
    }
}

#[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
fn print_table(result: &rosql::drivers::ROSQLResult, no_color: bool) {
    use tabled::{
        builder::Builder,
        settings::{object::Rows, Color, Modify, Style},
    };

    if result.rows.is_empty() {
        eprintln!("(0 rows)");
        return;
    }

    let headers: Vec<String> = result.columns.iter().map(|c| c.name.clone()).collect();

    let mut builder = Builder::default();
    builder.push_record(headers);

    for row in &result.rows {
        let cells: Vec<String> = row
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            })
            .collect();
        builder.push_record(cells);
    }

    let mut table = builder.build();
    table.with(Style::rounded());

    if !no_color {
        table.with(Modify::new(Rows::first()).with(Color::BOLD));
    }

    println!("{table}");

    let row_count = result.rows.len();
    let row_word = if row_count == 1 { "row" } else { "rows" };
    eprintln!(
        "({row_count} {row_word}, {}ms)",
        result.metadata.execution_time_ms
    );
}

#[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
fn print_csv(result: &rosql::drivers::ROSQLResult) {
    let headers: Vec<String> = result.columns.iter().map(|c| c.name.clone()).collect();
    println!("{}", csv_row(&headers));

    for row in &result.rows {
        let cells: Vec<String> = row
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            })
            .collect();
        println!("{}", csv_row(&cells));
    }
}

/// Encode a single CSV row, quoting fields that contain commas, quotes, or newlines.
#[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
fn csv_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| {
            if f.contains(',') || f.contains('"') || f.contains('\n') {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
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

fn read_query(positional: Option<String>, file: Option<std::path::PathBuf>) -> String {
    match (positional, file) {
        (Some(_), Some(_)) => {
            eprintln!("Error: provide either a query string or --file, not both.");
            std::process::exit(1);
        }
        (Some(q), None) => q,
        (None, Some(path)) => match std::fs::read_to_string(&path) {
            Ok(s) => s.trim().to_string(),
            Err(err) => {
                eprintln!("Error reading {}: {err}", path.display());
                std::process::exit(1);
            }
        },
        (None, None) => {
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
