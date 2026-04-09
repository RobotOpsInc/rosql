//! Recursive descent parser for ROSQL.
//!
//! Consumes a token stream from the lexer and produces a typed AST.
//! Multi-word keywords are combined here via lookahead.

use crate::ast::*;
use crate::error::ROSQLError;
use crate::lexer::{tokenize, Token};
use crate::span::{offset_to_location, SourceLocation};
use crate::units;

/// Parse a ROSQL source string into a typed AST.
pub fn parse(source: &str) -> Result<Query, Vec<ROSQLError>> {
    let tokens = tokenize(source).map_err(|offset| {
        vec![ROSQLError::ParseError {
            message: "unexpected character".into(),
            location: offset_to_location(source, offset),
            suggestion: None,
        }]
    })?;

    let mut parser = Parser::new(source, tokens);
    parser.parse_query().map_err(|e| vec![e])
}

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

struct Parser<'src> {
    source: &'src str,
    tokens: Vec<(Token<'src>, std::ops::Range<usize>)>,
    pos: usize,
}

impl<'src> Parser<'src> {
    fn new(source: &'src str, tokens: Vec<(Token<'src>, std::ops::Range<usize>)>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    // ── Navigation helpers ──────────────────────────────────────────

    fn peek(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek_second(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos + 1).map(|(t, _)| t)
    }

    fn advance(&mut self) -> Option<&Token<'src>> {
        let tok = self.tokens.get(self.pos).map(|(t, _)| t);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn current_location(&self) -> SourceLocation {
        if let Some((_, span)) = self.tokens.get(self.pos) {
            offset_to_location(self.source, span.start)
        } else {
            offset_to_location(self.source, self.source.len())
        }
    }

    fn expect(&mut self, expected: &Token<'_>) -> Result<(), ROSQLError> {
        if self.peek() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {expected:?}")))
        }
    }

    fn error(&self, message: String) -> ROSQLError {
        self.error_with_suggestion(message, None)
    }

    fn error_with_suggestion(&self, message: String, suggestion: Option<String>) -> ROSQLError {
        ROSQLError::ParseError {
            message,
            location: self.current_location(),
            suggestion,
        }
    }

    // ── Suggestion helpers ──────────────────────────────────────────

    fn suggest_keyword(&self, got: &str) -> Option<String> {
        const KEYWORDS: &[&str] = &[
            "SELECT",
            "FROM",
            "WHERE",
            "FOR",
            "ROBOT",
            "FLEET",
            "AND",
            "OR",
            "NOT",
            "AS",
            "ORDER",
            "BY",
            "ASC",
            "DESC",
            "LIMIT",
            "SINCE",
            "BETWEEN",
            "USING",
            "FACET",
            "FORMAT",
            "COMPARE",
            "TO",
            "LAST",
            "MESSAGE",
            "JOURNEY",
            "PATHS",
            "PATH",
            "DURING",
            "HEALTH",
            "ANOMALY",
            "DEVIATION",
            "CORRELATE",
            "SHOW",
            "RECORDING",
            "TRACE",
            "IS",
            "NULL",
            "IN",
            "LIKE",
            "HAVING",
            "WITH",
            "TIMESERIES",
            "ENRICH",
            "TOPICS",
            "NODES",
            "GRAPH",
            "SAMPLE",
            "FULL",
        ];

        let got_upper = got.to_uppercase();
        KEYWORDS
            .iter()
            .filter(|kw| {
                let dist = strsim::levenshtein(&got_upper, kw);
                dist > 0 && dist <= 2
            })
            .min_by_key(|kw| strsim::levenshtein(&got_upper, kw))
            .map(|kw| format!("did you mean: {kw}?"))
    }

    // ── Top-level dispatch ──────────────────────────────────────────

    fn parse_query(&mut self) -> Result<Query, ROSQLError> {
        // Check for mutation keywords first
        self.check_mutation_keywords()?;
        // Check for reserved keywords
        self.check_reserved_keywords()?;

        // Determine query type
        match self.peek() {
            // Compound clauses that start at the top level
            Some(Token::Message) => self.parse_compound_query(),
            Some(Token::Trace) if matches!(self.peek_second(), Some(Token::StringLiteral(_))) => {
                self.parse_compound_query()
            }
            Some(Token::Health) if matches!(self.peek_second(), Some(Token::LParen)) => {
                self.parse_compound_query()
            }
            Some(Token::Anomaly) => self.parse_compound_query(),
            Some(Token::Path) if matches!(self.peek_second(), Some(Token::Deviation)) => {
                self.parse_compound_query()
            }
            Some(Token::Joint) if matches!(self.peek_second(), Some(Token::Deviation)) => {
                self.parse_compound_query()
            }
            Some(Token::Correlate) => self.parse_compound_query(),
            Some(Token::Show) => self.parse_compound_query(),
            Some(Token::During) => self.parse_compound_query(),

            // Pipeline: FROM ... | ...
            Some(Token::From) if self.has_pipe() => {
                let pq = self.parse_pipeline_query()?;
                Ok(Query::Pipeline(pq))
            }

            // Standard query (SELECT ... FROM ... or FROM ...)
            _ => {
                let sq = self.parse_standard_query()?;
                Ok(Query::Standard(sq))
            }
        }
    }

    fn check_mutation_keywords(&self) -> Result<(), ROSQLError> {
        match self.peek() {
            Some(Token::Insert) | Some(Token::Update) | Some(Token::Delete) | Some(Token::Drop)
            | Some(Token::Create) => {
                let loc = self.current_location();
                let keyword = match self.peek().unwrap() {
                    Token::Insert => "INSERT",
                    Token::Update => "UPDATE",
                    Token::Delete => "DELETE",
                    Token::Drop => "DROP",
                    Token::Create => "CREATE",
                    _ => unreachable!(),
                };
                Err(ROSQLError::MutationRejected {
                    keyword: keyword.into(),
                    location: loc,
                })
            }
            _ => Ok(()),
        }
    }

    fn check_reserved_keywords(&self) -> Result<(), ROSQLError> {
        if matches!(self.peek(), Some(Token::Alert)) {
            let loc = self.current_location();
            return Err(ROSQLError::ReservedSyntax {
                keyword: "ALERT".into(),
                location: loc,
                message: "ALERT is a reserved keyword but is not supported in ROSQL. \
                           Alert rules should be configured in the Robot Ops platform. \
                           ROSQL is a read-only query language."
                    .into(),
            });
        }
        if matches!(self.peek(), Some(Token::Define)) {
            let loc = self.current_location();
            return Err(ROSQLError::ReservedSyntax {
                keyword: "DEFINE".into(),
                location: loc,
                message: "DEFINE is a reserved keyword but is not supported in ROSQL. \
                           Saved queries can be managed in the Robot Ops platform dashboard."
                    .into(),
            });
        }
        Ok(())
    }

    /// Check if there is a `|` token anywhere — indicates pipeline syntax.
    fn has_pipe(&self) -> bool {
        self.tokens[self.pos..]
            .iter()
            .any(|(t, _)| *t == Token::Pipe)
    }

    // ── Standard query ──────────────────────────────────────────────

    fn parse_standard_query(&mut self) -> Result<ROSQLQuery, ROSQLError> {
        let mut query = ROSQLQuery {
            selections: Vec::new(),
            data_source: DataSource::Logs, // placeholder
            scope: None,
            conditions: None,
            facet: None,
            time_range: None,
            time_basis: None,
            order_by: None,
            limit: None,
            offset: None,
            output_format: None,
            baseline: None,
            timeseries: None,
            enrichments: Vec::new(),
        };

        // Optional FOR ... scope clause(s) at the start
        while matches!(self.peek(), Some(Token::For)) {
            let scope = query.scope.get_or_insert_with(QueryScope::empty);
            self.parse_scope_clause(scope)?;
        }

        // SELECT or FROM
        if matches!(self.peek(), Some(Token::Select)) {
            self.advance(); // consume SELECT
            query.selections = self.parse_select_list()?;
            self.expect(&Token::From)?;
        } else if matches!(self.peek(), Some(Token::From)) {
            self.advance(); // consume FROM
            query.selections = vec![Selection::Star];
        } else {
            let got = self
                .peek()
                .map(|t| format!("{t:?}"))
                .unwrap_or("end of input".into());
            let suggestion = if let Some(Token::Identifier(id)) = self.peek() {
                self.suggest_keyword(id)
            } else {
                None
            };
            return Err(self
                .error_with_suggestion(format!("expected SELECT or FROM, got {got}"), suggestion));
        }

        query.data_source = self.parse_data_source()?;

        // Optional FOR ... scope clause(s) after FROM
        while matches!(self.peek(), Some(Token::For)) {
            let scope = query.scope.get_or_insert_with(QueryScope::empty);
            self.parse_scope_clause(scope)?;
        }

        // Optional clauses in any order
        loop {
            match self.peek() {
                Some(Token::Where) => {
                    self.advance();
                    query.conditions = Some(self.parse_condition()?);
                }
                Some(Token::Facet) => {
                    self.advance();
                    query.facet = Some(self.parse_facet()?);
                }
                Some(Token::Since) => {
                    query.time_range = Some(self.parse_since()?);
                }
                Some(Token::Between) => {
                    query.time_range = Some(self.parse_between()?);
                }
                Some(Token::Using) => {
                    self.advance();
                    query.time_basis = Some(self.parse_time_basis()?);
                }
                Some(Token::Order) => {
                    query.order_by = Some(self.parse_order_by()?);
                }
                Some(Token::Limit) => {
                    self.advance();
                    query.limit = Some(self.parse_limit_value()?);
                    // Support inline LIMIT N OFFSET M
                    if matches!(self.peek(), Some(Token::Offset)) {
                        self.advance();
                        query.offset = Some(self.parse_limit_value()?);
                    }
                }
                Some(Token::Offset) => {
                    self.advance();
                    query.offset = Some(self.parse_limit_value()?);
                }
                Some(Token::Format) => {
                    self.advance();
                    query.output_format = Some(self.parse_output_format()?);
                }
                Some(Token::Compare) => {
                    query.baseline = Some(self.parse_baseline()?);
                }
                Some(Token::For) => {
                    let scope = query.scope.get_or_insert_with(QueryScope::empty);
                    self.parse_scope_clause(scope)?;
                }
                Some(Token::Timeseries) => {
                    self.advance();
                    query.timeseries = Some(TimeseriesClause {
                        interval: self.parse_timeseries_interval()?,
                    });
                }
                Some(Token::Enrich) => {
                    self.advance();
                    self.expect(&Token::With)?;
                    query.enrichments.push(self.parse_enrichment_clause()?);
                }
                Some(Token::Semicolon) => {
                    self.advance();
                    break;
                }
                None => break,
                _ => break,
            }
        }

        Ok(query)
    }

    // ── Pipeline query ──────────────────────────────────────────────

    fn parse_pipeline_query(&mut self) -> Result<PipelineQuery, ROSQLError> {
        let mut stages = Vec::new();

        // First stage: FROM source
        self.expect(&Token::From)?;
        let source = self.parse_data_source()?;
        stages.push(PipelineStage::From(source));

        // Subsequent stages separated by |
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance(); // consume |
            let stage = self.parse_pipeline_stage()?;
            stages.push(stage);
        }

        // Consume optional trailing semicolon
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }

        Ok(PipelineQuery { stages })
    }

    fn parse_pipeline_stage(&mut self) -> Result<PipelineStage, ROSQLError> {
        match self.peek() {
            Some(Token::Where) => {
                self.advance();
                Ok(PipelineStage::Where(self.parse_condition()?))
            }
            Some(Token::Select) => {
                self.advance();
                Ok(PipelineStage::Select(self.parse_select_list()?))
            }
            Some(Token::Facet) => {
                self.advance();
                Ok(PipelineStage::Facet(self.parse_facet()?))
            }
            Some(Token::Since) => Ok(PipelineStage::Since(self.parse_since()?)),
            Some(Token::Between) => Ok(PipelineStage::Since(self.parse_between()?)),
            Some(Token::Using) => {
                self.advance();
                Ok(PipelineStage::Using(self.parse_time_basis()?))
            }
            Some(Token::Order) => Ok(PipelineStage::OrderBy(self.parse_order_by()?)),
            Some(Token::Limit) => {
                self.advance();
                Ok(PipelineStage::Limit(self.parse_limit_value()?))
            }
            Some(Token::Offset) => {
                self.advance();
                Ok(PipelineStage::Offset(self.parse_limit_value()?))
            }
            Some(Token::Format) => {
                self.advance();
                Ok(PipelineStage::Format(self.parse_output_format()?))
            }
            Some(Token::Compare) => Ok(PipelineStage::CompareTo(self.parse_baseline()?)),
            Some(Token::For) => {
                let mut scope = QueryScope::empty();
                while matches!(self.peek(), Some(Token::For)) {
                    self.parse_scope_clause(&mut scope)?;
                }
                Ok(PipelineStage::ForScope(scope))
            }
            Some(Token::Show) => {
                let clause = self.parse_show_clause()?;
                Ok(PipelineStage::CompoundClause(clause))
            }
            Some(Token::Timeseries) => {
                self.advance();
                Ok(PipelineStage::Timeseries(TimeseriesClause {
                    interval: self.parse_timeseries_interval()?,
                }))
            }
            Some(Token::Enrich) => {
                self.advance();
                self.expect(&Token::With)?;
                Ok(PipelineStage::EnrichWith(self.parse_enrichment_clause()?))
            }
            _ => {
                Err(self.error("expected pipeline stage (WHERE, SELECT, FACET, SINCE, TIMESERIES, ENRICH WITH, ...)".into()))
            }
        }
    }

    // ── Compound query ──────────────────────────────────────────────

    fn parse_compound_query(&mut self) -> Result<Query, ROSQLError> {
        let clause = self.parse_compound_clause()?;

        let mut cq = CompoundQuery {
            clause,
            scope: None,
            time_range: None,
            time_basis: None,
            conditions: None,
            facet: None,
            order_by: None,
            limit: None,
            offset: None,
            output_format: None,
            baseline: None,
        };

        // Parse trailing optional clauses
        loop {
            match self.peek() {
                Some(Token::For) => {
                    let scope = cq.scope.get_or_insert_with(QueryScope::empty);
                    self.parse_scope_clause(scope)?;
                }
                Some(Token::Since) => {
                    cq.time_range = Some(self.parse_since()?);
                }
                Some(Token::Between) => {
                    cq.time_range = Some(self.parse_between()?);
                }
                Some(Token::Using) => {
                    self.advance();
                    cq.time_basis = Some(self.parse_time_basis()?);
                }
                Some(Token::Where) => {
                    self.advance();
                    cq.conditions = Some(self.parse_condition()?);
                }
                Some(Token::Facet) => {
                    self.advance();
                    cq.facet = Some(self.parse_facet()?);
                }
                Some(Token::Order) => {
                    cq.order_by = Some(self.parse_order_by()?);
                }
                Some(Token::Limit) => {
                    self.advance();
                    cq.limit = Some(self.parse_limit_value()?);
                    // Support inline LIMIT N OFFSET M
                    if matches!(self.peek(), Some(Token::Offset)) {
                        self.advance();
                        cq.offset = Some(self.parse_limit_value()?);
                    }
                }
                Some(Token::Offset) => {
                    self.advance();
                    cq.offset = Some(self.parse_limit_value()?);
                }
                Some(Token::Format) => {
                    self.advance();
                    cq.output_format = Some(self.parse_output_format()?);
                }
                Some(Token::Compare) => {
                    cq.baseline = Some(self.parse_baseline()?);
                }
                Some(Token::Semicolon) => {
                    self.advance();
                    break;
                }
                None => break,
                _ => break,
            }
        }

        Ok(Query::Compound(cq))
    }

    fn parse_compound_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        match self.peek() {
            Some(Token::Message) => self.parse_message_clause(),
            Some(Token::Trace) => self.parse_trace_clause(),
            Some(Token::Health) => self.parse_health_clause(),
            Some(Token::Anomaly) => self.parse_anomaly_clause(),
            Some(Token::Path) => self.parse_path_deviation_clause(),
            Some(Token::Joint) => self.parse_joint_deviation_clause(),
            Some(Token::Correlate) => self.parse_correlate_clause(),
            Some(Token::Show) => self.parse_show_clause(),
            Some(Token::During) => self.parse_during_clause(),
            _ => Err(self.error("expected compound clause".into())),
        }
    }

    fn parse_message_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume MESSAGE
        match self.peek() {
            Some(Token::Flow) => {
                self.advance();
                // MESSAGE FLOW FROM TOPIC '/topic' [TO NODE '/node' | TO TOPIC '/topic'] [SHOW ...]
                self.expect(&Token::From)?;
                self.expect(&Token::Topic)?;
                let from_topic = self.parse_string_literal()?;
                let to_target = if matches!(self.peek(), Some(Token::To)) {
                    self.advance();
                    match self.peek() {
                        Some(Token::Node) => {
                            self.advance();
                            Some(FlowTarget::Node(self.parse_string_literal()?))
                        }
                        Some(Token::Topic) => {
                            self.advance();
                            Some(FlowTarget::Topic(self.parse_string_literal()?))
                        }
                        _ => {
                            return Err(self
                                .error("expected NODE or TOPIC after TO in MESSAGE FLOW".into()))
                        }
                    }
                } else {
                    None
                };
                let show = if matches!(self.peek(), Some(Token::Show)) {
                    self.advance();
                    Some(self.parse_identifier_string()?)
                } else {
                    None
                };
                Ok(CompoundClause::MessageFlow {
                    from_topic,
                    to_target,
                    show,
                })
            }
            Some(Token::Journey) => {
                self.advance();
                // Consume the old syntax to give a useful error
                let _ = self.expect(&Token::For);
                let _ = self.expect(&Token::Trace);
                let _ = self.parse_string_literal();
                Err(self.error(
                    "MESSAGE JOURNEY is removed. Use TRACE 'trace_id' instead \
                     (now performs a recursive span tree walk)."
                        .into(),
                ))
            }
            Some(Token::Paths) => {
                self.advance();
                let _ = self.expect(&Token::For);
                let _ = self.expect(&Token::Topic);
                let _ = self.parse_string_literal();
                Err(self.error(
                    "MESSAGE PATHS is removed. Use MESSAGE FLOW FROM TOPIC '/topic' instead."
                        .into(),
                ))
            }
            Some(Token::Path) => {
                self.advance();
                Err(self.error(
                    "MESSAGE PATH is removed. Use MESSAGE FLOW FROM TOPIC '/src' TO NODE '/dst' instead."
                        .into(),
                ))
            }
            _ => Err(self.error("expected FLOW after MESSAGE".into())),
        }
    }

    fn parse_trace_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume TRACE
        let trace_id = self.parse_string_literal()?;
        Ok(CompoundClause::Trace { trace_id })
    }

    fn parse_health_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume HEALTH
        self.expect(&Token::LParen)?;
        self.expect(&Token::RParen)?;
        Ok(CompoundClause::Health)
    }

    fn parse_anomaly_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume ANOMALY
        self.expect(&Token::LParen)?;
        let field = self.parse_identifier_string()?;
        self.expect(&Token::RParen)?;

        // Optional FROM <source>
        let data_source = if matches!(self.peek(), Some(Token::From)) {
            self.advance();
            Some(self.parse_data_source()?)
        } else {
            None
        };

        // COMPARED TO is now required
        if !matches!(self.peek(), Some(Token::Compared)) {
            return Err(ROSQLError::ParseError {
                message: "ANOMALY() requires COMPARED TO <baseline>".into(),
                location: self.current_location(),
                suggestion: Some(
                    "add COMPARED TO <baseline>, e.g. COMPARED TO last week, COMPARED TO fleet, COMPARED TO last 24 hours".into(),
                ),
            });
        }
        self.advance(); // consume COMPARED
        self.expect(&Token::To)?;
        let compared_to = self.parse_baseline_value()?;

        Ok(CompoundClause::Anomaly {
            field,
            compared_to,
            data_source,
        })
    }

    fn parse_path_deviation_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume PATH
        self.expect(&Token::Deviation)?;

        // Optional PLAN <integer> (e.g. PLAN 0 or PLAN -1)
        let plan_index = if matches!(self.peek(), Some(Token::Plan)) {
            self.advance(); // consume PLAN
            let neg = if matches!(self.peek(), Some(Token::Minus)) {
                self.advance();
                true
            } else {
                false
            };
            match self.peek() {
                Some(Token::Integer(s)) => {
                    let n: i64 = s
                        .parse()
                        .map_err(|_| self.error("expected integer after PLAN".into()))?;
                    self.advance();
                    Some(if neg { -n } else { n })
                }
                _ => return Err(self.error("expected integer after PLAN in PATH DEVIATION".into())),
            }
        } else {
            None
        };

        // Required FOR TRACE 'id' or FOR ROBOT 'id'
        self.expect(&Token::For)?;
        let target = match self.peek() {
            Some(Token::Trace) => {
                self.advance();
                DeviationTarget::Trace(self.parse_string_literal()?)
            }
            Some(Token::Robot) => {
                self.advance();
                DeviationTarget::Robot(self.parse_string_literal()?)
            }
            _ => {
                return Err(self.error("expected TRACE or ROBOT after FOR in PATH DEVIATION".into()))
            }
        };

        Ok(CompoundClause::PathDeviation { target, plan_index })
    }

    fn parse_joint_deviation_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume JOINT
        self.expect(&Token::Deviation)?;

        // Required FOR TRACE 'id' or FOR ROBOT 'id'
        self.expect(&Token::For)?;
        let target = match self.peek() {
            Some(Token::Trace) => {
                self.advance();
                DeviationTarget::Trace(self.parse_string_literal()?)
            }
            Some(Token::Robot) => {
                self.advance();
                DeviationTarget::Robot(self.parse_string_literal()?)
            }
            _ => {
                return Err(
                    self.error("expected TRACE or ROBOT after FOR in JOINT DEVIATION".into())
                )
            }
        };

        Ok(CompoundClause::JointDeviation { target })
    }

    fn parse_correlate_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume CORRELATE
        self.expect(&Token::With)?;
        let with_source = self.parse_data_source()?;
        Ok(CompoundClause::Correlate { with_source })
    }

    fn parse_show_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume SHOW
        match self.peek() {
            Some(Token::Recording) => {
                self.advance();
                Ok(CompoundClause::ShowRecording)
            }
            Some(Token::TraceBreakdown) => {
                self.advance();
                Ok(CompoundClause::ShowTraceBreakdown)
            }
            Some(Token::Deployments) => {
                self.advance();
                Ok(CompoundClause::ShowDeployments)
            }
            Some(Token::Span) => {
                self.advance();
                // SHOW SPAN SUMMARY
                match self.peek() {
                    Some(Token::Summary) => {
                        self.advance();
                        Ok(CompoundClause::ShowSpanSummary)
                    }
                    _ => Err(self.error("expected SUMMARY after SHOW SPAN".into())),
                }
            }
            Some(Token::Plans) => {
                self.advance();
                // SHOW PLANS [FOR TRACE 'id'] — consume FOR TRACE inline before trailing loop
                let trace_id =
                    if matches!(self.peek(), Some(Token::For))
                        && matches!(self.peek_second(), Some(Token::Trace))
                    {
                        self.advance(); // FOR
                        self.advance(); // TRACE
                        Some(self.parse_string_literal()?)
                    } else {
                        None
                    };
                Ok(CompoundClause::ShowPlans { trace_id })
            }
            Some(Token::Topics) => {
                self.advance();
                Ok(CompoundClause::ShowTopics)
            }
            Some(Token::Nodes) => {
                self.advance();
                Ok(CompoundClause::ShowNodes)
            }
            Some(Token::Node) => {
                self.advance();
                // SHOW NODE GRAPH
                match self.peek() {
                    Some(Token::Graph) => {
                        self.advance();
                        Ok(CompoundClause::ShowNodeGraph)
                    }
                    _ => Err(self.error("expected GRAPH after SHOW NODE".into())),
                }
            }
            Some(Token::Joints) => {
                self.advance();
                Ok(CompoundClause::ShowJoints)
            }
            _ => Err(self.error(
                "expected RECORDING, TRACE_BREAKDOWN, DEPLOYMENTS, SPAN SUMMARY, PLANS, TOPICS, NODES, NODE GRAPH, or JOINTS after SHOW"
                    .into(),
            )),
        }
    }

    fn parse_during_clause(&mut self) -> Result<CompoundClause, ROSQLError> {
        self.advance(); // consume DURING
        self.expect(&Token::LParen)?;

        // Parse inner subquery: FROM source WHERE conditions
        self.expect(&Token::From)?;
        let inner_source = self.parse_data_source()?;
        let inner_conditions = if matches!(self.peek(), Some(Token::Where)) {
            self.advance();
            Some(self.parse_condition()?)
        } else {
            None
        };
        let inner_time_range = if matches!(self.peek(), Some(Token::Since)) {
            Some(self.parse_since()?)
        } else {
            None
        };

        self.expect(&Token::RParen)?;
        Ok(CompoundClause::During {
            inner_source,
            inner_conditions,
            inner_time_range,
        })
    }

    // ── TIMESERIES / ENRICH WITH helpers ───────────────────────────

    fn parse_timeseries_interval(&mut self) -> Result<UnitValue, ROSQLError> {
        // Accept Integer/Float followed by either:
        //   - A SI unit symbol known to the unit registry (e.g. "min", "h", "ms")
        //   - A long-form English time word (e.g. "hours", "minutes", "day")
        let num_str = match self.peek() {
            Some(Token::Integer(s)) => {
                let s = s.to_string();
                self.advance();
                s
            }
            Some(Token::Float(s)) => {
                let s = s.to_string();
                self.advance();
                s
            }
            _ => {
                return Err(self.error(
                    "expected time interval after TIMESERIES (e.g. '5 min', '1 hour')".into(),
                ))
            }
        };

        let unit_sym = match self.peek() {
            Some(Token::Identifier(u)) if units::lookup_unit(u).is_some() => {
                let s = u.to_string();
                self.advance();
                s
            }
            Some(Token::Identifier(u)) if is_time_unit_word(&u.to_lowercase()) => {
                // Map long-form to SI symbol
                let mapped = match u.to_lowercase().as_str() {
                    "nanosecond" | "nanoseconds" => "ns",
                    "microsecond" | "microseconds" => "us",
                    "millisecond" | "milliseconds" => "ms",
                    "second" | "seconds" => "s",
                    "minute" | "minutes" => "min",
                    "hour" | "hours" => "h",
                    "day" | "days" => "days",
                    "week" | "weeks" => "days", // 1 week ≈ 7 days handled via raw_value
                    _ => "s",
                };
                self.advance();
                mapped.to_string()
            }
            _ => {
                return Err(self.error(
                    "expected time unit after TIMESERIES interval (e.g. 'min', 'hour', 'day')"
                        .into(),
                ))
            }
        };

        let raw: f64 = num_str
            .parse()
            .map_err(|_| self.error(format!("invalid number '{num_str}'")))?;
        let (si_val, si_unit) =
            units::convert_to_si(raw, &unit_sym, None).map_err(|e| match e {
                ROSQLError::UnitError { message, .. } => self.error(message),
                other => other,
            })?;

        Ok(UnitValue {
            raw_value: raw,
            unit: unit_sym,
            si_value: si_val,
            si_unit,
        })
    }

    fn parse_enrichment_clause(&mut self) -> Result<EnrichmentClause, ROSQLError> {
        let source = self.parse_data_source()?;
        let mut limit = None;
        let mut sample_full = false;

        // Parse optional LIMIT N and SAMPLE FULL modifiers in any order
        loop {
            match self.peek() {
                Some(Token::Limit) => {
                    self.advance();
                    limit = Some(self.parse_limit_value()?);
                }
                Some(Token::Sample) => {
                    self.advance();
                    self.expect(&Token::Full)?;
                    sample_full = true;
                }
                _ => break,
            }
        }

        Ok(EnrichmentClause {
            source,
            join_key: None,
            limit,
            sample_full,
        })
    }

    // ── SELECT list ─────────────────────────────────────────────────

    fn parse_select_list(&mut self) -> Result<Vec<Selection>, ROSQLError> {
        let mut selections = Vec::new();
        selections.push(self.parse_selection()?);
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            selections.push(self.parse_selection()?);
        }
        Ok(selections)
    }

    fn parse_selection(&mut self) -> Result<Selection, ROSQLError> {
        if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            return Ok(Selection::Star);
        }

        let sel = if self.is_aggregation_fn() {
            Selection::Aggregation(self.parse_aggregation_call()?)
        } else {
            let field = self.parse_dotted_identifier()?;
            Selection::Field(field)
        };

        // Check for AS alias
        if matches!(self.peek(), Some(Token::As)) {
            self.advance();
            let alias = self.parse_identifier_string()?;
            Ok(Selection::Aliased {
                expr: Box::new(sel),
                alias,
            })
        } else {
            Ok(sel)
        }
    }

    // ── Data source ─────────────────────────────────────────────────

    fn parse_data_source(&mut self) -> Result<DataSource, ROSQLError> {
        let name = self.parse_identifier_or_keyword_as_source()?;
        match name.to_lowercase().as_str() {
            "logs" => Ok(DataSource::Logs),
            "system_logs" => Ok(DataSource::SystemLogs),
            "traces" => Ok(DataSource::Traces),
            "metrics" => Ok(DataSource::Metrics),
            "diagnostics" => Ok(DataSource::Diagnostics),
            "topics" => Ok(DataSource::Topics),
            "tf" => Ok(DataSource::Tf),
            "heartbeats" => Ok(DataSource::Heartbeats),
            "recordings" => Ok(DataSource::Recordings),
            "events" => Ok(DataSource::Events),
            // Topic aliases
            "odom" => Ok(DataSource::TopicAlias(TopicAlias::Odom)),
            "joint_states" => Ok(DataSource::TopicAlias(TopicAlias::JointStates)),
            "battery" => Ok(DataSource::TopicAlias(TopicAlias::Battery)),
            "cmd_vel" => Ok(DataSource::TopicAlias(TopicAlias::CmdVel)),
            "imu" => Ok(DataSource::TopicAlias(TopicAlias::Imu)),
            _ => Err(self.error(format!("unknown data source '{name}'"))),
        }
    }

    /// Parse an identifier, but also accept certain keywords that are valid
    /// data source names (e.g. `traces` which could also be an identifier).
    fn parse_identifier_or_keyword_as_source(&mut self) -> Result<String, ROSQLError> {
        match self.peek() {
            Some(Token::Identifier(id)) => {
                let s = id.to_string();
                self.advance();
                Ok(s)
            }
            // Keywords that are also valid data source names
            Some(Token::Trace) => {
                self.advance();
                Ok("trace".to_string())
            }
            Some(Token::Topics) => {
                self.advance();
                Ok("topics".to_string())
            }
            Some(Token::Nodes) => {
                self.advance();
                Ok("nodes".to_string())
            }
            Some(Token::Node) => {
                self.advance();
                Ok("node".to_string())
            }
            Some(Token::Graph) => {
                self.advance();
                Ok("graph".to_string())
            }
            Some(Token::Recording) => {
                self.advance();
                Ok("recordings".to_string())
            }
            Some(Token::Session) => {
                self.advance();
                Ok("events".to_string())
            }
            _ => {
                // Try to consume any identifier-like token
                if let Some(tok) = self.peek() {
                    let s = format!("{tok:?}");
                    Err(self.error(format!("expected data source name, got {s}")))
                } else {
                    Err(self.error("expected data source name".into()))
                }
            }
        }
    }

    // ── Query scope ─────────────────────────────────────────────────

    /// Parse one `FOR <dimension> [value]` clause and merge into `scope`.
    fn parse_scope_clause(&mut self, scope: &mut QueryScope) -> Result<(), ROSQLError> {
        self.expect(&Token::For)?;
        match self.peek() {
            Some(Token::Robot) => {
                self.advance();
                let robot_id = self.parse_string_literal()?;
                scope.robot = Some(RobotScope::Single(robot_id));
            }
            Some(Token::Fleet) => {
                self.advance();
                scope.robot = Some(RobotScope::Fleet);
            }
            Some(Token::Version) => {
                self.advance();
                scope.version = Some(self.parse_string_literal()?);
            }
            Some(Token::Environment) => {
                self.advance();
                scope.environment = Some(self.parse_string_literal()?);
            }
            Some(Token::Session) => {
                self.advance();
                scope.session = Some(self.parse_string_literal()?);
            }
            _ => {
                return Err(self.error(
                    "expected ROBOT, FLEET, VERSION, ENVIRONMENT, or SESSION after FOR".into(),
                ))
            }
        }
        Ok(())
    }

    // ── Conditions ──────────────────────────────────────────────────

    fn parse_condition(&mut self) -> Result<Condition, ROSQLError> {
        self.parse_or_condition()
    }

    fn parse_or_condition(&mut self) -> Result<Condition, ROSQLError> {
        let mut left = self.parse_and_condition()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_and_condition()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and_condition(&mut self) -> Result<Condition, ROSQLError> {
        let mut left = self.parse_not_condition()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.advance();
            let right = self.parse_not_condition()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not_condition(&mut self) -> Result<Condition, ROSQLError> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.advance();
            let inner = self.parse_primary_condition()?;
            Ok(Condition::Not(Box::new(inner)))
        } else {
            self.parse_primary_condition()
        }
    }

    fn parse_primary_condition(&mut self) -> Result<Condition, ROSQLError> {
        // Parenthesised condition
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let cond = self.parse_condition()?;
            self.expect(&Token::RParen)?;
            return Ok(cond);
        }

        let left = self.parse_expr()?;

        // IS [NOT] NULL
        if matches!(self.peek(), Some(Token::Is)) {
            self.advance();
            if matches!(self.peek(), Some(Token::Not)) {
                self.advance();
                self.expect(&Token::Null)?;
                return Ok(Condition::IsNotNull(left));
            }
            self.expect(&Token::Null)?;
            return Ok(Condition::IsNull(left));
        }

        // IN (value, ...)
        if matches!(self.peek(), Some(Token::In)) {
            self.advance();
            self.expect(&Token::LParen)?;
            let mut values = vec![self.parse_expr()?];
            while matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                values.push(self.parse_expr()?);
            }
            self.expect(&Token::RParen)?;
            return Ok(Condition::In { expr: left, values });
        }

        // LIKE 'pattern'
        if matches!(self.peek(), Some(Token::Like)) {
            self.advance();
            let pattern = self.parse_string_literal()?;
            return Ok(Condition::Like {
                expr: left,
                pattern,
            });
        }

        // BETWEEN low AND high
        if matches!(self.peek(), Some(Token::Between)) {
            self.advance();
            let low = self.parse_expr()?;
            self.expect(&Token::And)?;
            let high = self.parse_expr()?;
            return Ok(Condition::Between {
                expr: left,
                low: Box::new(low),
                high: Box::new(high),
            });
        }

        // WITHIN <radius> OF [POSITION] (<x>, <y>)
        if matches!(self.peek(), Some(Token::Within)) {
            self.advance(); // consume WITHIN
                            // Parse the radius as a unit value expression, e.g. "500 m" or "2 m"
            let radius_expr = self.parse_primary_expr()?;
            let radius = match radius_expr {
                Expr::UnitValue(uv) => uv,
                _ => {
                    return Err(
                        self.error("expected a unit value after WITHIN (e.g. 500 m, 2 m)".into())
                    )
                }
            };
            self.expect(&Token::Of)?;
            let center = if matches!(self.peek(), Some(Token::Position)) {
                // Local frame: WITHIN N m OF POSITION (x, y)
                self.advance(); // consume POSITION
                self.expect(&Token::LParen)?;
                let x = self.parse_float_or_int()?;
                self.expect(&Token::Comma)?;
                let y = self.parse_float_or_int()?;
                self.expect(&Token::RParen)?;
                GeospatialCenter::Local(x, y)
            } else {
                // GPS: WITHIN N m OF (lat, lon)
                self.expect(&Token::LParen)?;
                let lat = self.parse_float_or_int()?;
                self.expect(&Token::Comma)?;
                let lon = self.parse_float_or_int()?;
                self.expect(&Token::RParen)?;
                GeospatialCenter::Gps(lat, lon)
            };
            return Ok(Condition::Within {
                field: left,
                radius,
                center,
            });
        }

        // Comparison operators
        let op = match self.peek() {
            Some(Token::Eq) => ComparisonOp::Eq,
            Some(Token::Neq) => ComparisonOp::Neq,
            Some(Token::Lt) => ComparisonOp::Lt,
            Some(Token::Gt) => ComparisonOp::Gt,
            Some(Token::Lte) => ComparisonOp::Lte,
            Some(Token::Gte) => ComparisonOp::Gte,
            _ => return Err(self.error("expected comparison operator".into())),
        };
        self.advance();

        let right = self.parse_expr()?;
        Ok(Condition::Comparison { left, op, right })
    }

    // ── Expressions ─────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, ROSQLError> {
        let left = self.parse_primary_expr()?;

        // Check for arithmetic operators
        match self.peek() {
            Some(Token::Plus | Token::Minus | Token::Star | Token::Slash) => {
                let op = match self.peek().unwrap() {
                    Token::Plus => ArithmeticOp::Add,
                    Token::Minus => ArithmeticOp::Sub,
                    Token::Star => ArithmeticOp::Mul,
                    Token::Slash => ArithmeticOp::Div,
                    _ => unreachable!(),
                };
                self.advance();
                let right = self.parse_primary_expr()?;
                Ok(Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }
            _ => Ok(left),
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ROSQLError> {
        match self.peek() {
            // Numeric literal — possibly followed by a unit suffix
            Some(Token::Integer(_)) => {
                let num_str = if let Some(Token::Integer(s)) = self.peek() {
                    s.to_string()
                } else {
                    unreachable!()
                };
                self.advance();

                // Check for unit suffix
                if let Some(Token::Identifier(maybe_unit)) = self.peek() {
                    if units::lookup_unit(maybe_unit).is_some() {
                        let unit_sym = maybe_unit.to_string();
                        self.advance();
                        let raw: f64 = num_str
                            .parse()
                            .map_err(|_| self.error(format!("invalid number '{num_str}'")))?;
                        let (si_val, si_unit) = units::convert_to_si(raw, &unit_sym, None)
                            .map_err(|e| match e {
                                ROSQLError::UnitError { message, .. } => self.error(message),
                                other => other,
                            })?;
                        return Ok(Expr::UnitValue(UnitValue {
                            raw_value: raw,
                            unit: unit_sym,
                            si_value: si_val,
                            si_unit,
                        }));
                    }
                }

                // Plain integer
                let val: i64 = num_str
                    .parse()
                    .map_err(|_| self.error(format!("invalid integer '{num_str}'")))?;
                Ok(Expr::Literal(Literal::Integer(val)))
            }

            Some(Token::Float(_)) => {
                let num_str = if let Some(Token::Float(s)) = self.peek() {
                    s.to_string()
                } else {
                    unreachable!()
                };
                self.advance();

                // Check for unit suffix
                if let Some(Token::Identifier(maybe_unit)) = self.peek() {
                    if units::lookup_unit(maybe_unit).is_some() {
                        let unit_sym = maybe_unit.to_string();
                        self.advance();
                        let raw: f64 = num_str
                            .parse()
                            .map_err(|_| self.error(format!("invalid number '{num_str}'")))?;
                        let (si_val, si_unit) = units::convert_to_si(raw, &unit_sym, None)
                            .map_err(|e| match e {
                                ROSQLError::UnitError { message, .. } => self.error(message),
                                other => other,
                            })?;
                        return Ok(Expr::UnitValue(UnitValue {
                            raw_value: raw,
                            unit: unit_sym,
                            si_value: si_val,
                            si_unit,
                        }));
                    }
                }

                let val: f64 = num_str
                    .parse()
                    .map_err(|_| self.error(format!("invalid float '{num_str}'")))?;
                Ok(Expr::Literal(Literal::Float(val)))
            }

            // String literal
            Some(Token::StringLiteral(_)) => {
                let s = self.parse_string_literal()?;
                Ok(Expr::Literal(Literal::String(s)))
            }

            // Boolean literals
            Some(Token::True) => {
                self.advance();
                Ok(Expr::Literal(Literal::Boolean(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::Literal(Literal::Boolean(false)))
            }

            // NULL
            Some(Token::Null) => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }

            // Aggregation function
            _ if self.is_aggregation_fn() => {
                let agg = self.parse_aggregation_call()?;
                Ok(Expr::Aggregation(agg))
            }

            // Parenthesised expression
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }

            // Identifier or keyword-as-identifier — field reference,
            // possibly dotted or bracket access
            _ if self.try_keyword_as_identifier().is_some()
                || matches!(self.peek(), Some(Token::Identifier(_))) =>
            {
                let first = self.parse_flexible_identifier()?;

                // Check for bracket access: fields['key']
                if matches!(self.peek(), Some(Token::LBracket)) {
                    self.advance(); // [
                    let key = self.parse_string_literal()?;
                    self.expect(&Token::RBracket)?;
                    return Ok(Expr::FieldAccess { base: first, key });
                }

                // Check for dotted identifier: ros.node
                let full = self.parse_dotted_rest(first)?;
                Ok(Expr::Field(full))
            }

            _ => {
                let got = self
                    .peek()
                    .map(|t| format!("{t:?}"))
                    .unwrap_or("end of input".into());
                Err(self.error(format!("expected expression, got {got}")))
            }
        }
    }

    // ── Aggregation functions ───────────────────────────────────────

    fn is_aggregation_fn(&self) -> bool {
        match self.peek() {
            Some(Token::Identifier(id)) => {
                matches!(
                    id.to_uppercase().as_str(),
                    "COUNT"
                        | "SUM"
                        | "AVG"
                        | "MIN"
                        | "MAX"
                        | "PERCENTILE"
                        | "STDDEV"
                        | "RATE"
                        | "DELTA"
                        | "DERIVATIVE"
                        | "MOVING_AVG"
                        | "TOPIC_RATE"
                        | "NODE_STATUS"
                        | "EXPECTED"
                        | "ACTION_SUCCESS_RATE"
                        | "UPTIME"
                        | "APPROX_COUNT_DISTINCT"
                        | "APPROX_PERCENTILE"
                ) && matches!(
                    self.tokens.get(self.pos + 1).map(|(t, _)| t),
                    Some(Token::LParen)
                )
            }
            _ => false,
        }
    }

    fn parse_aggregation_call(&mut self) -> Result<AggregationCall, ROSQLError> {
        let fn_name = self.parse_identifier_string_raw()?;
        let function = match fn_name.to_uppercase().as_str() {
            "COUNT" => AggregationFn::Count,
            "SUM" => AggregationFn::Sum,
            "AVG" => AggregationFn::Avg,
            "MIN" => AggregationFn::Min,
            "MAX" => AggregationFn::Max,
            "PERCENTILE" => AggregationFn::Percentile,
            "STDDEV" => AggregationFn::Stddev,
            "RATE" => AggregationFn::Rate,
            "DELTA" => AggregationFn::Delta,
            "DERIVATIVE" => AggregationFn::Derivative,
            "MOVING_AVG" => AggregationFn::MovingAvg,
            "TOPIC_RATE" => AggregationFn::TopicRate,
            "NODE_STATUS" => AggregationFn::NodeStatus,
            "EXPECTED" => AggregationFn::Expected,
            "ACTION_SUCCESS_RATE" => AggregationFn::ActionSuccessRate,
            "UPTIME" => AggregationFn::Uptime,
            "APPROX_COUNT_DISTINCT" => AggregationFn::ApproxCountDistinct,
            "APPROX_PERCENTILE" => AggregationFn::ApproxPercentile,
            _ => return Err(self.error(format!("unknown function '{fn_name}'"))),
        };

        self.expect(&Token::LParen)?;
        let mut args = Vec::new();

        if matches!(self.peek(), Some(Token::Star)) {
            // COUNT(*) — treat * as a special field
            self.advance();
            args.push(Expr::Field("*".into()));
        } else if !matches!(self.peek(), Some(Token::RParen)) {
            args.push(self.parse_expr()?);
            while matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }

        self.expect(&Token::RParen)?;
        Ok(AggregationCall { function, args })
    }

    // ── Time expressions ────────────────────────────────────────────

    fn parse_since(&mut self) -> Result<TimeRange, ROSQLError> {
        self.advance(); // consume SINCE
        let expr = self.parse_time_expr()?;
        Ok(TimeRange::Since(expr))
    }

    fn parse_between(&mut self) -> Result<TimeRange, ROSQLError> {
        self.advance(); // consume BETWEEN
        let start = self.parse_time_expr()?;
        self.expect(&Token::And)?;
        let end = self.parse_time_expr()?;
        Ok(TimeRange::Between { start, end })
    }

    fn parse_time_expr(&mut self) -> Result<TimeExpr, ROSQLError> {
        // Lifecycle anchor: LAST DEPLOYMENT / LAST ROBOT RESTART / etc.
        if matches!(self.peek(), Some(Token::Last)) {
            return self.parse_lifecycle_anchor_or_baseline_time();
        }

        // "yesterday"
        if matches!(self.peek(), Some(Token::Yesterday)) {
            self.advance();
            return Ok(TimeExpr::Relative(RelativeTime {
                amount: 1.0,
                unit: "days".into(),
            }));
        }

        // String literal → RFC 3339 timestamp
        if matches!(self.peek(), Some(Token::StringLiteral(_))) {
            let ts = self.parse_string_literal()?;
            return Ok(TimeExpr::Absolute(ts));
        }

        // Numeric: could be unix epoch or relative time
        if matches!(self.peek(), Some(Token::Integer(_) | Token::Float(_))) {
            let num_str = match self.peek() {
                Some(Token::Integer(s)) => s.to_string(),
                Some(Token::Float(s)) => s.to_string(),
                _ => unreachable!(),
            };
            self.advance();

            // Check for relative time unit words: "30 minutes ago"
            if let Some(Token::Identifier(unit_word)) = self.peek() {
                let unit_lower = unit_word.to_lowercase();
                if is_time_unit_word(&unit_lower) {
                    let unit = unit_word.to_string();
                    self.advance();
                    // Expect "ago"
                    if matches!(self.peek(), Some(Token::Ago)) {
                        self.advance();
                    }
                    let amount: f64 = num_str
                        .parse()
                        .map_err(|_| self.error(format!("invalid number '{num_str}'")))?;
                    return Ok(TimeExpr::Relative(RelativeTime { amount, unit }));
                }
            }

            // Check for time keywords that are tokens (not identifiers)
            if matches!(self.peek(), Some(Token::Identifier(_))) {
                // already handled above
            }

            // Unix epoch — disambiguate by digit count
            let digits = num_str.len();
            let val: u64 = num_str
                .parse()
                .map_err(|_| self.error(format!("invalid epoch timestamp '{num_str}'")))?;
            match digits {
                10 => return Ok(TimeExpr::Epoch(UnixEpoch::Seconds(val))),
                13 => return Ok(TimeExpr::Epoch(UnixEpoch::Milliseconds(val))),
                19 => return Ok(TimeExpr::Epoch(UnixEpoch::Nanoseconds(val))),
                _ => {
                    return Err(self.error(format!(
                        "ambiguous epoch timestamp '{num_str}': expected 10 (seconds), 13 (milliseconds), or 19 (nanoseconds) digits"
                    )));
                }
            }
        }

        Err(self.error("expected time expression".into()))
    }

    fn parse_lifecycle_anchor_or_baseline_time(&mut self) -> Result<TimeExpr, ROSQLError> {
        self.advance(); // consume LAST
        let anchor = match self.peek() {
            Some(Token::Deployment) => {
                self.advance();
                LifecycleAnchor::LastDeployment
            }
            Some(Token::Robot) => {
                self.advance();
                self.expect(&Token::Restart)?;
                LifecycleAnchor::LastRobotRestart
            }
            Some(Token::Action) => {
                self.advance();
                self.expect(&Token::Failure)?;
                LifecycleAnchor::LastActionFailure
            }
            Some(Token::Topic) => {
                self.advance();
                self.expect(&Token::Drop)?;
                LifecycleAnchor::LastTopicDrop
            }
            Some(Token::Diagnostic) => {
                self.advance();
                self.expect(&Token::Warning)?;
                LifecycleAnchor::LastDiagnosticWarning
            }
            _ => {
                return Err(self.error(
                    "expected DEPLOYMENT, ROBOT RESTART, ACTION FAILURE, TOPIC DROP, or DIAGNOSTIC WARNING after LAST".into(),
                ));
            }
        };
        Ok(TimeExpr::Anchor(anchor))
    }

    fn parse_time_basis(&mut self) -> Result<TimeBasis, ROSQLError> {
        match self.peek() {
            Some(Token::RosTime) => {
                self.advance();
                Ok(TimeBasis::RosTime)
            }
            Some(Token::WallTime) => {
                self.advance();
                Ok(TimeBasis::WallTime)
            }
            _ => Err(self.error("expected ROS_TIME or WALL_TIME".into())),
        }
    }

    // ── FACET ───────────────────────────────────────────────────────

    fn parse_facet(&mut self) -> Result<FacetClause, ROSQLError> {
        let dimension = self.parse_dotted_identifier()?;
        Ok(FacetClause { dimension })
    }

    // ── ORDER BY ────────────────────────────────────────────────────

    fn parse_order_by(&mut self) -> Result<OrderBy, ROSQLError> {
        self.expect(&Token::Order)?;
        self.expect(&Token::By)?;
        let field = self.parse_dotted_identifier()?;
        let direction = match self.peek() {
            Some(Token::Asc) => {
                self.advance();
                SortDirection::Asc
            }
            Some(Token::Desc) => {
                self.advance();
                SortDirection::Desc
            }
            _ => SortDirection::Asc, // default
        };
        Ok(OrderBy { field, direction })
    }

    // ── LIMIT ───────────────────────────────────────────────────────

    fn parse_limit_value(&mut self) -> Result<u64, ROSQLError> {
        match self.peek() {
            Some(Token::Integer(s)) => {
                let val: u64 = s
                    .parse()
                    .map_err(|_| self.error(format!("invalid LIMIT value '{s}'")))?;
                self.advance();
                Ok(val)
            }
            _ => Err(self.error("expected integer after LIMIT".into())),
        }
    }

    // ── FORMAT ──────────────────────────────────────────────────────

    fn parse_output_format(&mut self) -> Result<OutputFormat, ROSQLError> {
        let name = self.parse_identifier_string()?;
        match name.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "timeseries" => Ok(OutputFormat::Timeseries),
            "scalar" => Ok(OutputFormat::Scalar),
            "trace_tree" => Ok(OutputFormat::TraceTree),
            "graph" => Ok(OutputFormat::Graph),
            "path" => Ok(OutputFormat::Path),
            _ => Err(self.error(format!(
                "unknown format '{name}'. Expected: table, timeseries, scalar, trace_tree, graph, path"
            ))),
        }
    }

    // ── COMPARE TO ──────────────────────────────────────────────────

    fn parse_baseline(&mut self) -> Result<Baseline, ROSQLError> {
        self.advance(); // consume COMPARE

        // COMPARE ROBOTS (no TO)
        if matches!(self.peek(), Some(Token::Robots)) {
            self.advance();
            return Ok(Baseline::CompareRobots);
        }

        // COMPARE VERSION 'v1' TO VERSION 'v2'
        if matches!(self.peek(), Some(Token::Version)) {
            self.advance();
            let v1 = self.parse_string_literal()?;
            self.expect(&Token::To)?;
            self.expect(&Token::Version)?;
            let v2 = self.parse_string_literal()?;
            return Ok(Baseline::VersionPair(v1, v2));
        }

        self.expect(&Token::To)?;
        self.parse_baseline_value()
    }

    fn parse_baseline_value(&mut self) -> Result<Baseline, ROSQLError> {
        match self.peek() {
            Some(Token::Last) => {
                self.advance();
                match self.peek() {
                    Some(Token::Week) => {
                        self.advance();
                        Ok(Baseline::LastWeek)
                    }
                    Some(Token::Deployment) => {
                        self.advance();
                        Ok(Baseline::LastDeployment)
                    }
                    Some(Token::Integer(s)) if *s == "24" => {
                        self.advance(); // consume 24
                                        // Expect the identifier "hours" (case-insensitive)
                        match self.peek() {
                            Some(Token::Identifier(u)) if u.eq_ignore_ascii_case("hours") => {
                                self.advance();
                                Ok(Baseline::Last24Hours)
                            }
                            _ => Err(self.error(
                                "expected 'hours' after LAST 24 (e.g. LAST 24 HOURS)".into(),
                            )),
                        }
                    }
                    _ => {
                        Err(self.error("expected WEEK, DEPLOYMENT, or 24 HOURS after LAST".into()))
                    }
                }
            }
            Some(Token::Fleet) => {
                self.advance();
                Ok(Baseline::Fleet)
            }
            Some(Token::Robot) => {
                self.advance();
                let robot_id = self.parse_string_literal()?;
                Ok(Baseline::Robot(robot_id))
            }
            Some(Token::Version) => {
                self.advance();
                let v = self.parse_string_literal()?;
                Ok(Baseline::Version(v))
            }
            Some(Token::Identifier(id)) if id.eq_ignore_ascii_case("fleet") => {
                self.advance();
                Ok(Baseline::Fleet)
            }
            _ => Err(self.error(
                "expected baseline (LAST WEEK, FLEET, ROBOT '...', LAST DEPLOYMENT, VERSION '...')"
                    .into(),
            )),
        }
    }

    // ── Identifier helpers ──────────────────────────────────────────

    /// Parse a floating-point or integer literal as an `f64`. Handles optional
    /// leading minus sign (for negative coordinates, e.g. `-122.4194`).
    fn parse_float_or_int(&mut self) -> Result<f64, ROSQLError> {
        let neg = if matches!(self.peek(), Some(Token::Minus)) {
            self.advance();
            true
        } else {
            false
        };
        let v = match self.peek() {
            Some(Token::Float(s)) => {
                let val: f64 = s
                    .parse()
                    .map_err(|_| self.error("invalid float literal".into()))?;
                self.advance();
                val
            }
            Some(Token::Integer(s)) => {
                let val: f64 = s
                    .parse()
                    .map_err(|_| self.error("invalid integer literal".into()))?;
                self.advance();
                val
            }
            _ => return Err(self.error("expected a numeric literal".into())),
        };
        Ok(if neg { -v } else { v })
    }

    fn parse_string_literal(&mut self) -> Result<String, ROSQLError> {
        match self.peek() {
            Some(Token::StringLiteral(s)) => {
                let val = s.to_string();
                self.advance();
                Ok(val)
            }
            _ => Err(self.error("expected string literal".into())),
        }
    }

    /// Try to interpret the current token as an identifier string.
    /// Returns Some(name) for Identifier tokens and keyword tokens
    /// that can appear as field names. Does NOT advance.
    fn try_keyword_as_identifier(&self) -> Option<String> {
        if let Some((_, span)) = self.tokens.get(self.pos) {
            match self.peek() {
                Some(Token::Identifier(id)) => Some(id.to_string()),
                // Keywords that commonly appear as field/source names
                Some(
                    Token::Action
                    | Token::Node
                    | Token::Topic
                    | Token::Trace
                    | Token::Recording
                    | Token::Health
                    | Token::Path
                    | Token::Message
                    | Token::Warning
                    | Token::Drop
                    | Token::Deployment
                    | Token::Restart
                    | Token::Failure
                    | Token::Diagnostic
                    | Token::Deviation
                    // New tokens that may appear as field names
                    | Token::Position
                    | Token::Joint
                    | Token::Joints
                    | Token::Plan
                    | Token::Of,
                ) => Some(self.source[span.clone()].to_string()),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Parse an identifier, also accepting keyword tokens that can serve
    /// as identifiers in field/expression positions.
    fn parse_flexible_identifier(&mut self) -> Result<String, ROSQLError> {
        if let Some(name) = self.try_keyword_as_identifier() {
            self.advance();
            Ok(name)
        } else {
            let got = self
                .peek()
                .map(|t| format!("{t:?}"))
                .unwrap_or("end of input".into());
            let suggestion = if let Some(Token::Identifier(id)) = self.peek() {
                self.suggest_keyword(id)
            } else {
                None
            };
            Err(self.error_with_suggestion(format!("expected identifier, got {got}"), suggestion))
        }
    }

    /// Parse an identifier token (raw, no dots). Only accepts Identifier tokens.
    fn parse_identifier_string_raw(&mut self) -> Result<String, ROSQLError> {
        match self.peek() {
            Some(Token::Identifier(id)) => {
                let s = id.to_string();
                self.advance();
                Ok(s)
            }
            _ => {
                let got = self
                    .peek()
                    .map(|t| format!("{t:?}"))
                    .unwrap_or("end of input".into());
                Err(self.error(format!("expected identifier, got {got}")))
            }
        }
    }

    /// Parse an identifier, accepting keyword tokens as identifiers.
    fn parse_identifier_string(&mut self) -> Result<String, ROSQLError> {
        self.parse_flexible_identifier()
    }

    /// Parse a potentially dotted identifier (e.g. `ros.node`).
    fn parse_dotted_identifier(&mut self) -> Result<String, ROSQLError> {
        let first = self.parse_identifier_string()?;
        self.parse_dotted_rest(first)
    }

    fn parse_dotted_rest(&mut self, first: String) -> Result<String, ROSQLError> {
        let mut result = first;
        while matches!(self.peek(), Some(Token::Dot)) {
            self.advance(); // consume .
                            // After a dot, accept keyword tokens as identifiers (e.g. ros.node)
            let next = self.parse_flexible_identifier()?;
            result.push('.');
            result.push_str(&next);
        }
        Ok(result)
    }

    fn parse_identifier_list(&mut self) -> Result<Vec<String>, ROSQLError> {
        let mut ids = vec![self.parse_dotted_identifier()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            ids.push(self.parse_dotted_identifier()?);
        }
        Ok(ids)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_time_unit_word(s: &str) -> bool {
    matches!(
        s,
        "nanoseconds"
            | "nanosecond"
            | "ns"
            | "microseconds"
            | "microsecond"
            | "us"
            | "milliseconds"
            | "millisecond"
            | "ms"
            | "seconds"
            | "second"
            | "s"
            | "minutes"
            | "minute"
            | "min"
            | "hours"
            | "hour"
            | "h"
            | "days"
            | "day"
            | "d"
            | "weeks"
            | "week"
            | "w"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Query {
        parse(source).unwrap_or_else(|errs| {
            panic!("parse failed for '{source}': {errs:?}");
        })
    }

    fn parse_err(source: &str) -> Vec<ROSQLError> {
        parse(source).unwrap_err()
    }

    // ── Basic queries ───────────────────────────────────────────────

    #[test]
    fn select_star_from_logs() {
        let q = parse_ok("SELECT * FROM logs");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.selections, vec![Selection::Star]);
                assert_eq!(sq.data_source, DataSource::Logs);
            }
            _ => panic!("expected Standard query"),
        }
    }

    #[test]
    fn from_shorthand() {
        let q = parse_ok("FROM logs");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.selections, vec![Selection::Star]);
                assert_eq!(sq.data_source, DataSource::Logs);
            }
            _ => panic!("expected Standard query"),
        }
    }

    #[test]
    fn select_fields() {
        let q = parse_ok("SELECT span_name, duration FROM logs");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.selections.len(), 2);
                assert_eq!(sq.selections[0], Selection::Field("span_name".into()));
                assert_eq!(sq.selections[1], Selection::Field("duration".into()));
            }
            _ => panic!("expected Standard query"),
        }
    }

    #[test]
    fn select_with_alias() {
        let q = parse_ok("SELECT AVG(duration) AS avg_dur FROM logs");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.selections[0], Selection::Aliased { .. }));
            }
            _ => panic!("expected Standard query"),
        }
    }

    // ── Data sources ────────────────────────────────────────────────

    #[test]
    fn all_data_sources() {
        for (name, expected) in [
            ("logs", DataSource::Logs),
            ("system_logs", DataSource::SystemLogs),
            ("metrics", DataSource::Metrics),
            ("diagnostics", DataSource::Diagnostics),
            ("topics", DataSource::Topics),
            ("tf", DataSource::Tf),
            ("heartbeats", DataSource::Heartbeats),
            ("recordings", DataSource::Recordings),
            ("events", DataSource::Events),
        ] {
            let q = parse_ok(&format!("FROM {name}"));
            match q {
                Query::Standard(sq) => assert_eq!(sq.data_source, expected, "source: {name}"),
                _ => panic!("expected Standard for {name}"),
            }
        }
    }

    #[test]
    fn topic_aliases() {
        for (name, expected) in [
            ("odom", TopicAlias::Odom),
            ("joint_states", TopicAlias::JointStates),
            ("battery", TopicAlias::Battery),
            ("cmd_vel", TopicAlias::CmdVel),
            ("imu", TopicAlias::Imu),
        ] {
            let q = parse_ok(&format!("FROM {name}"));
            match q {
                Query::Standard(sq) => {
                    assert_eq!(
                        sq.data_source,
                        DataSource::TopicAlias(expected),
                        "alias: {name}"
                    );
                }
                _ => panic!("expected Standard for {name}"),
            }
        }
    }

    // ── WHERE conditions ────────────────────────────────────────────

    #[test]
    fn where_comparison() {
        let q = parse_ok("FROM logs WHERE duration > 500");
        match q {
            Query::Standard(sq) => {
                assert!(sq.conditions.is_some());
                match sq.conditions.unwrap() {
                    Condition::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Gt),
                    _ => panic!("expected Comparison"),
                }
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_and_or() {
        let q = parse_ok("FROM logs WHERE a = 1 AND b = 2 OR c = 3");
        match q {
            Query::Standard(sq) => {
                // OR has lower precedence: (a=1 AND b=2) OR (c=3)
                assert!(matches!(sq.conditions, Some(Condition::Or(_, _))));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_not() {
        let q = parse_ok("FROM logs WHERE NOT a = 1");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.conditions, Some(Condition::Not(_))));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_is_null() {
        let q = parse_ok("FROM logs WHERE a IS NULL");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.conditions, Some(Condition::IsNull(_))));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_is_not_null() {
        let q = parse_ok("FROM logs WHERE a IS NOT NULL");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.conditions, Some(Condition::IsNotNull(_))));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_in() {
        let q = parse_ok("FROM logs WHERE severity IN ('ERROR', 'WARN')");
        match q {
            Query::Standard(sq) => match sq.conditions.unwrap() {
                Condition::In { values, .. } => assert_eq!(values.len(), 2),
                _ => panic!("expected In"),
            },
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn where_like() {
        let q = parse_ok("FROM logs WHERE message LIKE '%error%'");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.conditions, Some(Condition::Like { .. })));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Unit values ─────────────────────────────────────────────────

    #[test]
    fn unit_value_ms() {
        let q = parse_ok("FROM logs WHERE duration > 500 ms");
        match q {
            Query::Standard(sq) => {
                if let Some(Condition::Comparison { right, .. }) = sq.conditions {
                    match right {
                        Expr::UnitValue(uv) => {
                            assert_eq!(uv.raw_value, 500.0);
                            assert_eq!(uv.unit, "ms");
                            assert!((uv.si_value - 0.5).abs() < 1e-9);
                            assert_eq!(uv.si_unit, "s");
                        }
                        _ => panic!("expected UnitValue, got {right:?}"),
                    }
                } else {
                    panic!("expected comparison condition");
                }
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn unit_value_km() {
        let q = parse_ok("FROM logs WHERE distance > 5 km");
        match q {
            Query::Standard(sq) => {
                if let Some(Condition::Comparison { right, .. }) = sq.conditions {
                    match right {
                        Expr::UnitValue(uv) => {
                            assert_eq!(uv.raw_value, 5.0);
                            assert_eq!(uv.unit, "km");
                            assert!((uv.si_value - 5000.0).abs() < 1e-6);
                            assert_eq!(uv.si_unit, "m");
                        }
                        _ => panic!("expected UnitValue"),
                    }
                }
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Time expressions ────────────────────────────────────────────

    #[test]
    fn since_relative() {
        let q = parse_ok("FROM logs SINCE 30 minutes ago");
        match q {
            Query::Standard(sq) => match sq.time_range.unwrap() {
                TimeRange::Since(TimeExpr::Relative(rt)) => {
                    assert_eq!(rt.amount, 30.0);
                    assert_eq!(rt.unit, "minutes");
                }
                other => panic!("expected Relative, got {other:?}"),
            },
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_yesterday() {
        let q = parse_ok("FROM logs SINCE yesterday");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Relative(_)))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_absolute() {
        let q = parse_ok("FROM logs SINCE '2026-03-18T14:00:00Z'");
        match q {
            Query::Standard(sq) => match sq.time_range.unwrap() {
                TimeRange::Since(TimeExpr::Absolute(ts)) => {
                    assert_eq!(ts, "2026-03-18T14:00:00Z");
                }
                other => panic!("expected Absolute, got {other:?}"),
            },
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_unix_seconds() {
        let q = parse_ok("FROM logs SINCE 1742306400");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Epoch(UnixEpoch::Seconds(
                        1742306400
                    ))))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_unix_ms() {
        let q = parse_ok("FROM logs SINCE 1742306400000");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Epoch(UnixEpoch::Milliseconds(
                        1742306400000
                    ))))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_unix_ns() {
        let q = parse_ok("FROM logs SINCE 1742306400000000000");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Epoch(UnixEpoch::Nanoseconds(
                        1742306400000000000
                    ))))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn between_absolute() {
        let q = parse_ok("FROM logs BETWEEN '2026-03-18T14:00:00Z' AND '2026-03-18T15:00:00Z'");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(sq.time_range, Some(TimeRange::Between { .. })));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Lifecycle anchors ───────────────────────────────────────────

    #[test]
    fn since_last_deployment() {
        let q = parse_ok("FROM logs SINCE last deployment");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Anchor(
                        LifecycleAnchor::LastDeployment
                    )))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_last_robot_restart() {
        let q = parse_ok("FROM logs SINCE last robot restart");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Anchor(
                        LifecycleAnchor::LastRobotRestart
                    )))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_last_action_failure() {
        let q = parse_ok("FROM logs SINCE last action failure");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Anchor(
                        LifecycleAnchor::LastActionFailure
                    )))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_last_topic_drop() {
        let q = parse_ok("FROM logs SINCE last topic drop");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Anchor(
                        LifecycleAnchor::LastTopicDrop
                    )))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn since_last_diagnostic_warning() {
        let q = parse_ok("FROM logs SINCE last diagnostic warning");
        match q {
            Query::Standard(sq) => {
                assert!(matches!(
                    sq.time_range,
                    Some(TimeRange::Since(TimeExpr::Anchor(
                        LifecycleAnchor::LastDiagnosticWarning
                    )))
                ));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Time basis ──────────────────────────────────────────────────

    #[test]
    fn using_ros_time() {
        let q = parse_ok("FROM logs SINCE 10 minutes ago USING ROS_TIME");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.time_basis, Some(TimeBasis::RosTime));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Query scope ─────────────────────────────────────────────────

    #[test]
    fn for_robot() {
        let q = parse_ok("FOR ROBOT 'robot_42' FROM logs SINCE 1 hour ago");
        match q {
            Query::Standard(sq) => {
                let scope = sq.scope.unwrap();
                assert_eq!(scope.robot, Some(RobotScope::Single("robot_42".into())));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn for_fleet() {
        let q = parse_ok("FROM metrics SINCE 1 hour ago FOR FLEET");
        match q {
            Query::Standard(sq) => {
                let scope = sq.scope.unwrap();
                assert_eq!(scope.robot, Some(RobotScope::Fleet));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn composable_scope() {
        let q = parse_ok(
            "FOR ROBOT 'r1' FOR VERSION 'v2.3.1' FOR ENVIRONMENT 'production' FROM traces",
        );
        match q {
            Query::Standard(sq) => {
                let scope = sq.scope.unwrap();
                assert_eq!(scope.robot, Some(RobotScope::Single("r1".into())));
                assert_eq!(scope.version, Some("v2.3.1".into()));
                assert_eq!(scope.environment, Some("production".into()));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn for_session() {
        let q = parse_ok("FROM traces FOR SESSION 'sess_042'");
        match q {
            Query::Standard(sq) => {
                let scope = sq.scope.unwrap();
                assert_eq!(scope.session, Some("sess_042".into()));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── ORDER BY, LIMIT ─────────────────────────────────────────────

    #[test]
    fn order_by_desc() {
        let q = parse_ok("FROM logs ORDER BY duration DESC");
        match q {
            Query::Standard(sq) => {
                let ob = sq.order_by.unwrap();
                assert_eq!(ob.field, "duration");
                assert_eq!(ob.direction, SortDirection::Desc);
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn limit_value() {
        let q = parse_ok("FROM logs LIMIT 10");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.limit, Some(10));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── FACET ───────────────────────────────────────────────────────

    #[test]
    fn facet_clause() {
        let q = parse_ok("FROM logs FACET robot_id");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.facet.unwrap().dimension, "robot_id");
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── COMPARE TO ──────────────────────────────────────────────────

    #[test]
    fn compare_to_last_week() {
        let q = parse_ok("FROM logs COMPARE TO last week");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.baseline, Some(Baseline::LastWeek));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn compare_to_fleet() {
        let q = parse_ok("FROM logs COMPARE TO fleet");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.baseline, Some(Baseline::Fleet));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn compare_robots() {
        let q = parse_ok("FROM logs COMPARE ROBOTS");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.baseline, Some(Baseline::CompareRobots));
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Pipeline syntax ─────────────────────────────────────────────

    #[test]
    fn pipeline_basic() {
        let q = parse_ok("FROM logs | WHERE duration > 500 | FACET robot_id");
        match q {
            Query::Pipeline(pq) => {
                assert_eq!(pq.stages.len(), 3);
                assert!(matches!(
                    pq.stages[0],
                    PipelineStage::From(DataSource::Logs)
                ));
                assert!(matches!(pq.stages[1], PipelineStage::Where(_)));
                assert!(matches!(pq.stages[2], PipelineStage::Facet(_)));
            }
            _ => panic!("expected Pipeline"),
        }
    }

    #[test]
    fn pipeline_with_compare() {
        let q =
            parse_ok("FROM logs | WHERE duration > 500 ms | FACET robot_id | COMPARE TO last week");
        match q {
            Query::Pipeline(pq) => {
                assert_eq!(pq.stages.len(), 4);
                assert!(matches!(
                    pq.stages[3],
                    PipelineStage::CompareTo(Baseline::LastWeek)
                ));
            }
            _ => panic!("expected Pipeline"),
        }
    }

    // ── Compound clauses ────────────────────────────────────────────

    #[test]
    fn message_journey_deprecated() {
        let errs = parse_err("MESSAGE JOURNEY FOR TRACE 'abc123'");
        assert!(matches!(errs[0], ROSQLError::ParseError { .. }));
        let msg = errs[0].to_string();
        assert!(msg.contains("MESSAGE JOURNEY is removed"), "got: {msg}");
    }

    #[test]
    fn message_paths_deprecated() {
        let errs = parse_err("MESSAGE PATHS FOR TOPIC '/cmd_vel'");
        assert!(matches!(errs[0], ROSQLError::ParseError { .. }));
        let msg = errs[0].to_string();
        assert!(msg.contains("MESSAGE PATHS is removed"), "got: {msg}");
    }

    #[test]
    fn message_path_deprecated() {
        let errs = parse_err("MESSAGE PATH FROM TOPIC '/scan' TO NODE '/local_costmap_node'");
        assert!(matches!(errs[0], ROSQLError::ParseError { .. }));
        let msg = errs[0].to_string();
        assert!(msg.contains("MESSAGE PATH is removed"), "got: {msg}");
    }

    #[test]
    fn message_flow_from_topic() {
        let q = parse_ok("MESSAGE FLOW FROM TOPIC '/cmd_vel' SINCE 1 hour ago");
        match q {
            Query::Compound(cq) => {
                match &cq.clause {
                    CompoundClause::MessageFlow {
                        from_topic,
                        to_target,
                        ..
                    } => {
                        assert_eq!(from_topic, "/cmd_vel");
                        assert!(to_target.is_none());
                    }
                    _ => panic!("expected MessageFlow"),
                }
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn message_flow_to_node() {
        let q = parse_ok("MESSAGE FLOW FROM TOPIC '/scan' TO NODE '/local_costmap_node'");
        match q {
            Query::Compound(cq) => match &cq.clause {
                CompoundClause::MessageFlow {
                    from_topic,
                    to_target,
                    ..
                } => {
                    assert_eq!(from_topic, "/scan");
                    assert_eq!(
                        *to_target,
                        Some(FlowTarget::Node("/local_costmap_node".into()))
                    );
                }
                _ => panic!("expected MessageFlow"),
            },
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn message_flow_to_topic() {
        let q = parse_ok("MESSAGE FLOW FROM TOPIC '/cmd_vel' TO TOPIC '/motor_cmd'");
        match q {
            Query::Compound(cq) => match &cq.clause {
                CompoundClause::MessageFlow {
                    from_topic,
                    to_target,
                    ..
                } => {
                    assert_eq!(from_topic, "/cmd_vel");
                    assert_eq!(*to_target, Some(FlowTarget::Topic("/motor_cmd".into())));
                }
                _ => panic!("expected MessageFlow"),
            },
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn trace_query() {
        let q = parse_ok("TRACE 'abc123def456'");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(
                    cq.clause,
                    CompoundClause::Trace { ref trace_id } if trace_id == "abc123def456"
                ));
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn health_query() {
        let q = parse_ok("HEALTH() FOR ROBOT 'robot_42' SINCE 1 hour ago");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::Health));
                let scope = cq.scope.unwrap();
                assert_eq!(scope.robot, Some(RobotScope::Single("robot_42".into())));
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn anomaly_query() {
        let q = parse_ok("ANOMALY(duration) COMPARED TO fleet SINCE 24 hours ago FACET robot_id");
        match q {
            Query::Compound(cq) => {
                match &cq.clause {
                    CompoundClause::Anomaly {
                        field, compared_to, ..
                    } => {
                        assert_eq!(field, "duration");
                        assert_eq!(*compared_to, Baseline::Fleet);
                    }
                    _ => panic!("expected Anomaly"),
                }
                assert!(cq.time_range.is_some());
                assert!(cq.facet.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn path_deviation() {
        let q = parse_ok("PATH DEVIATION FOR TRACE 'trace-002'");
        match q {
            Query::Compound(cq) => match &cq.clause {
                CompoundClause::PathDeviation { target, plan_index } => {
                    assert_eq!(*target, DeviationTarget::Trace("trace-002".into()));
                    assert_eq!(*plan_index, None);
                }
                _ => panic!("expected PathDeviation"),
            },
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn correlate_query() {
        let q = parse_ok("CORRELATE WITH metrics");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(
                    cq.clause,
                    CompoundClause::Correlate {
                        with_source: DataSource::Metrics
                    }
                ));
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn show_recording() {
        let q = parse_ok("SHOW RECORDING SINCE yesterday");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::ShowRecording));
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn show_deployments() {
        let q = parse_ok("SHOW DEPLOYMENTS FOR ROBOT 'robot_42' SINCE 7 days ago");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::ShowDeployments));
                assert!(cq.scope.unwrap().robot.is_some());
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn show_span_summary() {
        let q = parse_ok("SHOW SPAN SUMMARY SINCE 1 hour ago");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::ShowSpanSummary));
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn show_plans_for_trace() {
        let q = parse_ok("SHOW PLANS FOR TRACE 'abc123'");
        match q {
            Query::Compound(cq) => match &cq.clause {
                CompoundClause::ShowPlans { trace_id } => {
                    assert_eq!(*trace_id, Some("abc123".into()));
                }
                _ => panic!("expected ShowPlans"),
            },
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn show_plans_for_robot() {
        let q = parse_ok("SHOW PLANS FOR ROBOT 'robot_42' SINCE 1 hour ago");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(
                    cq.clause,
                    CompoundClause::ShowPlans { trace_id: None }
                ));
                assert!(cq.scope.unwrap().robot.is_some());
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn compare_to_version() {
        let q = parse_ok("FROM traces COMPARE TO VERSION 'v2.3.1'");
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.baseline, Some(Baseline::Version("v2.3.1".into())));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn compare_version_pair() {
        let q = parse_ok("FROM traces COMPARE VERSION 'v1.0' TO VERSION 'v2.0'");
        match q {
            Query::Standard(sq) => {
                assert_eq!(
                    sq.baseline,
                    Some(Baseline::VersionPair("v1.0".into(), "v2.0".into()))
                );
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Aggregation functions ───────────────────────────────────────

    #[test]
    fn aggregation_count() {
        let q = parse_ok("SELECT COUNT(*) FROM logs");
        match q {
            Query::Standard(sq) => match &sq.selections[0] {
                Selection::Aggregation(agg) => {
                    assert_eq!(agg.function, AggregationFn::Count);
                }
                _ => panic!("expected Aggregation"),
            },
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn aggregation_percentile() {
        let q = parse_ok("SELECT PERCENTILE(duration, 95) FROM logs");
        match q {
            Query::Standard(sq) => match &sq.selections[0] {
                Selection::Aggregation(agg) => {
                    assert_eq!(agg.function, AggregationFn::Percentile);
                    assert_eq!(agg.args.len(), 2);
                }
                _ => panic!("expected Aggregation"),
            },
            _ => panic!("expected Standard"),
        }
    }

    // ── Field access ────────────────────────────────────────────────

    #[test]
    fn bracket_field_access() {
        let q = parse_ok("FROM logs WHERE fields['my_value'] > 42");
        match q {
            Query::Standard(sq) => {
                if let Some(Condition::Comparison { left, .. }) = sq.conditions {
                    match left {
                        Expr::FieldAccess { base, key } => {
                            assert_eq!(base, "fields");
                            assert_eq!(key, "my_value");
                        }
                        _ => panic!("expected FieldAccess"),
                    }
                }
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn dotted_field() {
        let q = parse_ok("FROM logs WHERE ros.node = '/planner'");
        match q {
            Query::Standard(sq) => {
                if let Some(Condition::Comparison { left, .. }) = sq.conditions {
                    assert_eq!(left, Expr::Field("ros.node".into()));
                }
            }
            _ => panic!("expected Standard"),
        }
    }

    // ── Error cases ─────────────────────────────────────────────────

    #[test]
    fn mutation_insert_rejected() {
        let errs = parse_err("INSERT INTO logs VALUES (1)");
        assert!(matches!(errs[0], ROSQLError::MutationRejected { .. }));
    }

    #[test]
    fn mutation_update_rejected() {
        let errs = parse_err("UPDATE logs SET x = 1");
        assert!(matches!(errs[0], ROSQLError::MutationRejected { .. }));
    }

    #[test]
    fn mutation_delete_rejected() {
        let errs = parse_err("DELETE FROM logs");
        assert!(matches!(errs[0], ROSQLError::MutationRejected { .. }));
    }

    #[test]
    fn mutation_drop_rejected() {
        let errs = parse_err("DROP logs");
        // DROP is also the lifecycle anchor keyword, but at top level it's a mutation
        assert!(matches!(errs[0], ROSQLError::MutationRejected { .. }));
    }

    #[test]
    fn mutation_create_rejected() {
        let errs = parse_err("CREATE TABLE logs (id INT)");
        assert!(matches!(errs[0], ROSQLError::MutationRejected { .. }));
    }

    #[test]
    fn reserved_alert() {
        let errs = parse_err("ALERT WHEN cpu > 90");
        assert!(matches!(errs[0], ROSQLError::ReservedSyntax { .. }));
    }

    #[test]
    fn reserved_define() {
        let errs = parse_err("DEFINE SLO availability 99.9");
        assert!(matches!(errs[0], ROSQLError::ReservedSyntax { .. }));
    }

    #[test]
    fn did_you_mean_suggestion() {
        let errs = parse_err("SELCT * FROM logs");
        match &errs[0] {
            ROSQLError::ParseError { suggestion, .. } => {
                assert!(suggestion.is_some());
                assert!(suggestion.as_ref().unwrap().contains("SELECT"));
            }
            _ => panic!("expected ParseError with suggestion"),
        }
    }

    // ── Complex queries from the spec ───────────────────────────────

    #[test]
    fn full_standard_query() {
        let q = parse_ok(
            "SELECT span_name, duration FROM logs \
             WHERE duration > 500 ms \
             FACET robot_id \
             SINCE 30 minutes ago \
             ORDER BY duration DESC \
             LIMIT 10",
        );
        match q {
            Query::Standard(sq) => {
                assert_eq!(sq.selections.len(), 2);
                assert_eq!(sq.data_source, DataSource::Logs);
                assert!(sq.conditions.is_some());
                assert!(sq.facet.is_some());
                assert!(sq.time_range.is_some());
                assert!(sq.order_by.is_some());
                assert_eq!(sq.limit, Some(10));
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn health_faceted() {
        let q = parse_ok("HEALTH() SINCE 30 minutes ago FACET robot_id");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::Health));
                assert!(cq.time_range.is_some());
                assert_eq!(cq.facet.unwrap().dimension, "robot_id");
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn trace_with_scope() {
        let q = parse_ok("TRACE 'abc123' FOR ROBOT 'r1'");
        match q {
            Query::Compound(cq) => {
                assert!(matches!(cq.clause, CompoundClause::Trace { .. }));
                let scope = cq.scope.unwrap();
                assert_eq!(scope.robot, Some(RobotScope::Single("r1".into())));
            }
            _ => panic!("expected Compound"),
        }
    }
}
