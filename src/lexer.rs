//! Lexer for ROSQL — tokenises input text into a stream of `Token`s.
//!
//! Built on `logos` for zero-copy tokenisation. Multi-word keywords
//! (e.g. MESSAGE JOURNEY, PATH DEVIATION) are emitted as separate tokens
//! and combined by the parser.

use logos::Logos;

/// All tokens in the ROSQL language.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"--[^\n]*")]
pub enum Token<'src> {
    // ── Keywords (case-insensitive) ─────────────────────────────────
    #[token("SELECT", ignore(ascii_case))]
    Select,
    #[token("FROM", ignore(ascii_case))]
    From,
    #[token("WHERE", ignore(ascii_case))]
    Where,
    #[token("FOR", ignore(ascii_case))]
    For,
    #[token("ROBOT", ignore(ascii_case))]
    Robot,
    #[token("FLEET", ignore(ascii_case))]
    Fleet,
    #[token("AND", ignore(ascii_case))]
    And,
    #[token("OR", ignore(ascii_case))]
    Or,
    #[token("NOT", ignore(ascii_case))]
    Not,
    #[token("AS", ignore(ascii_case))]
    As,
    #[token("ORDER", ignore(ascii_case))]
    Order,
    #[token("BY", ignore(ascii_case))]
    By,
    #[token("ASC", ignore(ascii_case))]
    Asc,
    #[token("DESC", ignore(ascii_case))]
    Desc,
    #[token("LIMIT", ignore(ascii_case))]
    Limit,
    #[token("SINCE", ignore(ascii_case))]
    Since,
    #[token("BETWEEN", ignore(ascii_case))]
    Between,
    #[token("USING", ignore(ascii_case))]
    Using,
    #[token("FACET", ignore(ascii_case))]
    Facet,
    #[token("FORMAT", ignore(ascii_case))]
    Format,
    #[token("COMPARE", ignore(ascii_case))]
    Compare,
    #[token("TO", ignore(ascii_case))]
    To,
    #[token("LAST", ignore(ascii_case))]
    Last,
    #[token("IS", ignore(ascii_case))]
    Is,
    #[token("NULL", ignore(ascii_case))]
    Null,
    #[token("IN", ignore(ascii_case))]
    In,
    #[token("LIKE", ignore(ascii_case))]
    Like,
    #[token("HAVING", ignore(ascii_case))]
    Having,
    #[token("WITH", ignore(ascii_case))]
    With,

    // ── Compound clause keywords (combined by parser) ───────────────
    #[token("MESSAGE", ignore(ascii_case))]
    Message,
    #[token("JOURNEY", ignore(ascii_case))]
    Journey,
    #[token("FLOW", ignore(ascii_case))]
    Flow,
    #[token("PATHS", ignore(ascii_case))]
    Paths,
    #[token("PATH", ignore(ascii_case))]
    Path,
    #[token("DURING", ignore(ascii_case))]
    During,
    #[token("HEALTH", ignore(ascii_case))]
    Health,
    #[token("ANOMALY", ignore(ascii_case))]
    Anomaly,
    #[token("DEVIATION", ignore(ascii_case))]
    Deviation,
    #[token("CORRELATE", ignore(ascii_case))]
    Correlate,
    #[token("SHOW", ignore(ascii_case))]
    Show,
    #[token("RECORDING", ignore(ascii_case))]
    Recording,
    #[token("TRACE", ignore(ascii_case))]
    Trace,
    #[token("COMPARED", ignore(ascii_case))]
    Compared,
    #[token("ROBOTS", ignore(ascii_case))]
    Robots,
    #[token("JOINT", ignore(ascii_case))]
    Joint,
    #[token("JOINTS", ignore(ascii_case))]
    Joints,
    #[token("WITHIN", ignore(ascii_case))]
    Within,
    #[token("OF", ignore(ascii_case))]
    Of,
    #[token("POSITION", ignore(ascii_case))]
    Position,
    #[token("PLAN", ignore(ascii_case))]
    Plan,

    // ── Time-related keywords ───────────────────────────────────────
    #[token("AGO", ignore(ascii_case))]
    Ago,
    #[token("YESTERDAY", ignore(ascii_case))]
    Yesterday,
    #[token("WEEK", ignore(ascii_case))]
    Week,
    #[token("DEPLOYMENT", ignore(ascii_case))]
    Deployment,
    #[token("DEPLOYMENTS", ignore(ascii_case))]
    Deployments,
    #[token("RESTART", ignore(ascii_case))]
    Restart,
    #[token("FAILURE", ignore(ascii_case))]
    Failure,
    #[token("DROP", ignore(ascii_case))]
    Drop,
    #[token("DIAGNOSTIC", ignore(ascii_case))]
    Diagnostic,
    #[token("WARNING", ignore(ascii_case))]
    Warning,
    #[token("ACTION", ignore(ascii_case))]
    Action,
    #[token("TOPIC", ignore(ascii_case))]
    Topic,
    #[token("NODE", ignore(ascii_case))]
    Node,
    #[token("VERSION", ignore(ascii_case))]
    Version,
    #[token("ENVIRONMENT", ignore(ascii_case))]
    Environment,
    #[token("SESSION", ignore(ascii_case))]
    Session,
    #[token("PLANS", ignore(ascii_case))]
    Plans,
    #[token("SPAN", ignore(ascii_case))]
    Span,
    #[token("SUMMARY", ignore(ascii_case))]
    Summary,
    #[token("TOPICS", ignore(ascii_case))]
    Topics,
    #[token("NODES", ignore(ascii_case))]
    Nodes,
    #[token("GRAPH", ignore(ascii_case))]
    Graph,
    #[token("TIMESERIES", ignore(ascii_case))]
    Timeseries,
    #[token("ENRICH", ignore(ascii_case))]
    Enrich,
    #[token("SAMPLE", ignore(ascii_case))]
    Sample,
    #[token("FULL", ignore(ascii_case))]
    Full,

    // ── Time basis ──────────────────────────────────────────────────
    #[token("ROS_TIME", ignore(ascii_case))]
    RosTime,
    #[token("WALL_TIME", ignore(ascii_case))]
    WallTime,

    // ── Boolean literals ────────────────────────────────────────────
    #[token("TRUE", ignore(ascii_case))]
    True,
    #[token("FALSE", ignore(ascii_case))]
    False,

    // ── Mutation keywords (rejected by parser) ──────────────────────
    #[token("INSERT", ignore(ascii_case))]
    Insert,
    #[token("UPDATE", ignore(ascii_case))]
    Update,
    #[token("DELETE", ignore(ascii_case))]
    Delete,
    #[token("CREATE", ignore(ascii_case))]
    Create,

    // ── Reserved keywords (future versions) ─────────────────────────
    #[token("ALERT", ignore(ascii_case))]
    Alert,
    #[token("DEFINE", ignore(ascii_case))]
    Define,
    #[token("SLO", ignore(ascii_case))]
    Slo,
    #[token("WHEN", ignore(ascii_case))]
    When,

    // ── AT keyword (for PATH DEVIATION ... AT '...') ────────────────
    #[token("AT", ignore(ascii_case))]
    At,

    // ── OFFSET keyword ──────────────────────────────────────────────
    #[token("OFFSET", ignore(ascii_case))]
    Offset,

    // ── TRACE_BREAKDOWN ─────────────────────────────────────────────
    #[token("TRACE_BREAKDOWN", ignore(ascii_case))]
    TraceBreakdown,

    // ── Literals ────────────────────────────────────────────────────
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice(), priority = 3)]
    Float(&'src str),

    #[regex(r"[0-9]+", |lex| lex.slice(), priority = 2)]
    Integer(&'src str),

    #[regex(r"'[^']*'", |lex| &lex.slice()[1..lex.slice().len()-1])]
    StringLiteral(&'src str),

    // ── Operators ───────────────────────────────────────────────────
    #[token("!=")]
    Neq,
    #[token("<=")]
    Lte,
    #[token(">=")]
    Gte,
    #[token("=")]
    Eq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,

    // ── Punctuation ─────────────────────────────────────────────────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token("|")]
    Pipe,
    #[token("*")]
    Star,
    #[token(";")]
    Semicolon,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(".")]
    Dot,

    // ── Identifiers (must be last to avoid shadowing keywords) ──────
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice(), priority = 1)]
    Identifier(&'src str),
}

/// A token paired with its byte span in the source text.
pub type Spanned<'src> = (Token<'src>, std::ops::Range<usize>);

/// Tokenise a ROSQL source string into a vector of spanned tokens.
/// Returns an error with the byte offset of any unrecognised character.
pub fn tokenize(source: &str) -> Result<Vec<Spanned<'_>>, usize> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(source);

    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => tokens.push((token, lexer.span())),
            Err(()) => return Err(lexer.span().start),
        }
    }

    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(source: &str) -> Vec<Token<'_>> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    #[test]
    fn basic_select() {
        assert_eq!(
            tok("SELECT * FROM logs"),
            vec![
                Token::Select,
                Token::Star,
                Token::From,
                Token::Identifier("logs")
            ]
        );
    }

    #[test]
    fn case_insensitive_keywords() {
        assert_eq!(
            tok("select * from LOGS"),
            vec![
                Token::Select,
                Token::Star,
                Token::From,
                Token::Identifier("LOGS")
            ]
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            tok("500 3.14"),
            vec![Token::Integer("500"), Token::Float("3.14")]
        );
    }

    #[test]
    fn string_literal() {
        assert_eq!(
            tok("'hello world'"),
            vec![Token::StringLiteral("hello world")]
        );
    }

    #[test]
    fn operators() {
        assert_eq!(
            tok("= != < > <= >="),
            vec![
                Token::Eq,
                Token::Neq,
                Token::Lt,
                Token::Gt,
                Token::Lte,
                Token::Gte
            ]
        );
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            tok("( ) , | * ; [ ]"),
            vec![
                Token::LParen,
                Token::RParen,
                Token::Comma,
                Token::Pipe,
                Token::Star,
                Token::Semicolon,
                Token::LBracket,
                Token::RBracket
            ]
        );
    }

    #[test]
    fn pipeline_syntax() {
        let tokens = tok("FROM traces | WHERE duration > 500 | FACET robot_id");
        assert_eq!(tokens[0], Token::From);
        assert_eq!(tokens[2], Token::Pipe);
        assert_eq!(tokens[3], Token::Where);
        assert_eq!(tokens[7], Token::Pipe);
        assert_eq!(tokens[8], Token::Facet);
    }

    #[test]
    fn compound_keywords_separate() {
        // Multi-word keywords are separate tokens
        assert_eq!(tok("MESSAGE JOURNEY"), vec![Token::Message, Token::Journey]);
        assert_eq!(tok("PATH DEVIATION"), vec![Token::Path, Token::Deviation]);
        assert_eq!(tok("SHOW RECORDING"), vec![Token::Show, Token::Recording]);
    }

    #[test]
    fn unit_suffix_as_identifier() {
        // "500 ms" → Integer + Identifier (parser resolves to UnitValue)
        assert_eq!(
            tok("500 ms"),
            vec![Token::Integer("500"), Token::Identifier("ms")]
        );
    }

    #[test]
    fn lifecycle_anchor_tokens() {
        assert_eq!(tok("LAST DEPLOYMENT"), vec![Token::Last, Token::Deployment]);
        assert_eq!(
            tok("LAST ROBOT RESTART"),
            vec![Token::Last, Token::Robot, Token::Restart]
        );
        assert_eq!(
            tok("LAST ACTION FAILURE"),
            vec![Token::Last, Token::Action, Token::Failure]
        );
    }

    #[test]
    fn mutation_keywords() {
        assert_eq!(tok("INSERT"), vec![Token::Insert]);
        assert_eq!(tok("UPDATE"), vec![Token::Update]);
        assert_eq!(tok("DELETE"), vec![Token::Delete]);
        assert_eq!(tok("DROP"), vec![Token::Drop]);
        assert_eq!(tok("CREATE"), vec![Token::Create]);
    }

    #[test]
    fn reserved_keywords() {
        assert_eq!(tok("ALERT"), vec![Token::Alert]);
        assert_eq!(tok("DEFINE"), vec![Token::Define]);
        assert_eq!(tok("SLO"), vec![Token::Slo]);
    }

    #[test]
    fn time_basis() {
        assert_eq!(tok("ROS_TIME"), vec![Token::RosTime]);
        assert_eq!(tok("WALL_TIME"), vec![Token::WallTime]);
    }

    #[test]
    fn comment_skipped() {
        assert_eq!(
            tok("SELECT * -- this is a comment\nFROM logs"),
            vec![
                Token::Select,
                Token::Star,
                Token::From,
                Token::Identifier("logs")
            ]
        );
    }

    #[test]
    fn full_query() {
        let q = "SELECT span_name, duration FROM traces WHERE duration > 500 SINCE 1 AGO LIMIT 10";
        let tokens = tok(q);
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Identifier("span_name"));
        assert_eq!(tokens[2], Token::Comma);
        assert_eq!(tokens[3], Token::Identifier("duration"));
        assert_eq!(tokens[4], Token::From);
        assert_eq!(tokens[5], Token::Identifier("traces"));
        assert_eq!(tokens[6], Token::Where);
    }

    #[test]
    fn for_robot_scope() {
        assert_eq!(
            tok("FOR ROBOT 'robot_42'"),
            vec![Token::For, Token::Robot, Token::StringLiteral("robot_42")]
        );
        assert_eq!(tok("FOR FLEET"), vec![Token::For, Token::Fleet]);
    }

    #[test]
    fn compare_to_baseline() {
        assert_eq!(
            tok("COMPARE TO LAST WEEK"),
            vec![Token::Compare, Token::To, Token::Last, Token::Week]
        );
        assert_eq!(tok("COMPARE ROBOTS"), vec![Token::Compare, Token::Robots]);
    }

    #[test]
    fn dotted_field_as_separate_tokens() {
        // "ros.node" → Identifier("ros") + Dot + Node
        // Parser reconstructs the dotted path, treating keywords as identifiers.
        assert_eq!(
            tok("ros.node"),
            vec![Token::Identifier("ros"), Token::Dot, Token::Node]
        );
    }

    #[test]
    fn bracket_field_access() {
        assert_eq!(
            tok("fields['key']"),
            vec![
                Token::Identifier("fields"),
                Token::LBracket,
                Token::StringLiteral("key"),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn health_compound() {
        assert_eq!(
            tok("HEALTH()"),
            vec![Token::Health, Token::LParen, Token::RParen]
        );
    }

    #[test]
    fn anomaly_compound() {
        assert_eq!(
            tok("ANOMALY(duration)"),
            vec![
                Token::Anomaly,
                Token::LParen,
                Token::Identifier("duration"),
                Token::RParen
            ]
        );
    }

    #[test]
    fn trace_breakdown() {
        assert_eq!(
            tok("SHOW TRACE_BREAKDOWN"),
            vec![Token::Show, Token::TraceBreakdown]
        );
    }

    #[test]
    fn arithmetic_operators() {
        assert_eq!(
            tok("a + b - c"),
            vec![
                Token::Identifier("a"),
                Token::Plus,
                Token::Identifier("b"),
                Token::Minus,
                Token::Identifier("c"),
            ]
        );
    }
}
