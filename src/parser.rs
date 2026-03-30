//! GQL — Geometric Query Language v2.0 parser (§6).
//!
//! Maps geometric statements to GIGI engine operations:
//!
//!   **GQL Native:**
//!   BUNDLE name BASE (...) FIBER (...) → create bundle
//!   SECTION name (...) → insert record
//!   SECTIONS name (...) → batch insert
//!   SECTION name AT k=v → point query (O(1))
//!   SECTION name AT k=v PROJECT (...) → projected point query
//!   REDEFINE name AT k=v SET (...) → update
//!   RETRACT name AT k=v → delete
//!   COVER name ON f=v → range query (bitmap, O(|bucket|))
//!   COVER name WHERE cond → filtered query (scan, O(|n|))
//!   COVER name DISTINCT f → distinct values
//!   COVER name ALL → list all
//!   INTEGRATE name OVER f MEASURE agg(g) → GROUP BY aggregation
//!   PULLBACK name ALONG f ONTO name → join
//!   CURVATURE name → scalar curvature
//!   SPECTRAL name → spectral gap
//!   CONSISTENCY name → Čech H¹
//!   EXPLAIN (...) → query plan
//!   SHOW BUNDLES → list bundles
//!   DESCRIBE name → schema info
//!   COLLAPSE name → drop bundle
//!   HEALTH name → full diagnostic
//!   EXISTS SECTION name AT k=v → existence check
//!   ATLAS BEGIN / COMMIT / ROLLBACK → transaction control
//!
//!   **SQL Compat (backward-compatible):**
//!   CREATE BUNDLE → BUNDLE
//!   INSERT INTO → SECTION
//!   SELECT → COVER / SECTION AT

use std::collections::HashMap;

/// Parsed GQL statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // ── Schema ──
    CreateBundle {
        name: String,
        base_fields: Vec<FieldSpec>,
        fiber_fields: Vec<FieldSpec>,
        indexed: Vec<String>,
        encrypted: bool,
        adjacencies: Vec<AdjacencySpec>,
    },
    Collapse {
        bundle: String,
    },
    Describe {
        bundle: String,
        verbose: bool,
    },
    ShowBundles,

    // ── Write ──
    Insert {
        bundle: String,
        columns: Vec<String>,
        values: Vec<Literal>,
    },
    BatchInsert {
        bundle: String,
        columns: Vec<String>,
        rows: Vec<Vec<Literal>>,
    },
    SectionUpsert {
        bundle: String,
        columns: Vec<String>,
        values: Vec<Literal>,
    },
    Redefine {
        bundle: String,
        key: Vec<(String, Literal)>,
        sets: Vec<(String, Literal)>,
    },
    BulkRedefine {
        bundle: String,
        conditions: Vec<FilterCondition>,
        sets: Vec<(String, Literal)>,
    },
    Retract {
        bundle: String,
        key: Vec<(String, Literal)>,
    },
    BulkRetract {
        bundle: String,
        conditions: Vec<FilterCondition>,
    },

    // ── Point Query ──
    PointQuery {
        bundle: String,
        key: Vec<(String, Literal)>,
        project: Option<Vec<String>>,
    },
    ExistsSection {
        bundle: String,
        key: Vec<(String, Literal)>,
    },

    // ── Range/Cover Query ──
    Cover {
        bundle: String,
        on_conditions: Vec<FilterCondition>,
        where_conditions: Vec<FilterCondition>,
        or_groups: Vec<Vec<FilterCondition>>,
        distinct_field: Option<String>,
        project: Option<Vec<String>>,
        rank_by: Option<Vec<SortSpec>>,
        first: Option<usize>,
        skip: Option<usize>,
        all: bool,
    },

    // ── Aggregation ──
    Integrate {
        bundle: String,
        over: Option<String>,
        measures: Vec<MeasureSpec>,
    },

    // ── Joins ──
    Pullback {
        left: String,
        along: String,
        right: String,
        right_field: Option<String>,
        preserve_left: bool,
    },

    // ── SQL Compat: SELECT ──
    Select {
        bundle: String,
        columns: Vec<SelectCol>,
        condition: Option<Condition>,
        group_by: Option<String>,
    },

    // ── SQL Compat: JOIN ──
    Join {
        left: String,
        right: String,
        on_field: String,
        columns: Vec<SelectCol>,
    },

    // ── Analytics ──
    Curvature {
        bundle: String,
        fields: Vec<String>,
        by_field: Option<String>,
    },
    Spectral {
        bundle: String,
        full: bool,
    },
    Consistency {
        bundle: String,
        repair: bool,
    },
    /// COMPLETE ON bundle [WHERE ...] [METHOD ...] [MIN_CONFIDENCE n] [WITH ...]
    Complete {
        bundle: String,
        where_conditions: Vec<FilterCondition>,
        method: Option<String>,
        min_confidence: Option<f64>,
        with_provenance: bool,
        with_constraint_graph: bool,
    },
    /// PROPAGATE ON bundle ASSUMING key=val, key=val [SHOW NEWLY_DETERMINED]
    Propagate {
        bundle: String,
        assumptions: Vec<(String, Literal)>,
    },
    /// SUGGEST_ADJACENCY ON bundle [FIELDS f1,f2,...] [SAMPLE_SIZE n] [CANDIDATES k] MINIMIZING h1
    SuggestAdjacency {
        bundle: String,
        fields: Vec<String>,
        sample_size: usize,
        candidates: usize,
    },
    Health {
        bundle: String,
    },
    Explain {
        inner: Box<Statement>,
    },

    // ── Transaction ──
    AtlasBegin,
    AtlasCommit,
    AtlasRollback,

    // ── v2.1: Access Control ──
    WeaveRole {
        name: String,
        password: Option<String>,
        inherits: Option<String>,
        superweave: bool,
    },
    UnweaveRole {
        name: String,
    },
    ShowRoles,
    Grant {
        operations: Vec<String>,
        bundle: String,
        role: String,
    },
    Revoke {
        operations: Vec<String>,
        bundle: String,
        role: String,
    },
    CreatePolicy {
        name: String,
        bundle: String,
        operations: Vec<String>,
        restrict_query: String,
        role: String,
    },
    DropPolicy {
        name: String,
        bundle: String,
    },
    ShowPolicies {
        bundle: String,
    },
    AuditOn {
        bundle: String,
        operations: Vec<String>,
    },
    AuditOff {
        bundle: String,
    },
    AuditShow {
        bundle: String,
        since: Option<String>,
        role: Option<String>,
    },

    // ── v2.1: Constraints ──
    GaugeConstrain {
        bundle: String,
        constraints: Vec<String>,
    },
    GaugeUnconstrain {
        bundle: String,
        constraint_name: String,
    },
    ShowConstraints {
        bundle: String,
    },

    // ── v2.1: Maintenance ──
    Compact {
        bundle: String,
        analyze: bool,
    },
    Analyze {
        bundle: String,
        field: Option<String>,
        full: bool,
    },
    Vacuum {
        bundle: String,
        full: bool,
    },
    RebuildIndex {
        bundle: String,
        field: Option<String>,
    },
    CheckIntegrity {
        bundle: String,
    },
    Repair {
        bundle: String,
    },
    StorageInfo {
        bundle: String,
    },

    // ── v2.1: Session ──
    Set {
        key: String,
        value: Literal,
    },
    Reset {
        key: Option<String>,
    },
    ShowSettings,
    ShowSession,
    ShowCurrentRole,

    // ── v2.1: Data Movement ──
    Ingest {
        bundle: String,
        source: String,
        format: String,
    },
    Transplant {
        source: String,
        target: String,
        conditions: Vec<FilterCondition>,
        retract_source: bool,
    },
    GenerateBase {
        bundle: String,
        field: String,
        from_val: Literal,
        to_val: Literal,
        step: Literal,
    },
    Fill {
        bundle: String,
        field: String,
        method: String,
    },

    // ── v2.1: Prepared Statements ──
    Prepare {
        name: String,
        body: String,
    },
    Execute {
        name: String,
        params: Vec<Literal>,
    },
    Deallocate {
        name: Option<String>,
    },
    ShowPrepared,

    // ── v2.1: Backup / Restore ──
    Backup {
        bundle: Option<String>,
        path: String,
        compress: bool,
        incremental_since: Option<String>,
    },
    Restore {
        bundle: String,
        path: String,
        snapshot: Option<String>,
        rename: Option<String>,
    },
    VerifyBackup {
        path: String,
    },
    ShowBackups,

    // ── v2.1: Information Schema ──
    ShowFields {
        bundle: String,
    },
    ShowIndexes {
        bundle: String,
    },
    ShowMorphisms {
        bundle: String,
    },
    ShowTriggers {
        bundle: String,
    },
    ShowStatistics {
        bundle: String,
    },
    ShowGeometry {
        bundle: String,
    },
    ShowComments {
        bundle: String,
    },

    // ── v2.1: Comments ──
    CommentOn {
        target_type: String,
        target: String,
        comment: String,
    },

    // ── v2.1: Recursive ──
    Iterate {
        bundle: String,
        start_key: Vec<(String, Literal)>,
        step_field: String,
        max_depth: Option<usize>,
    },

    // ── v2.1: Triggers ──
    CreateTrigger {
        event: String,
        bundle: String,
        condition: Option<String>,
        action: String,
    },
    DropTrigger {
        name: String,
        bundle: String,
    },

    // ── Feature #6: Query Cache ──
    /// INVALIDATE CACHE [ON <bundle>]
    InvalidateCache {
        bundle: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub name: String,
    pub ftype: String,
    pub range: Option<f64>,
    pub default: Option<Literal>,
    pub auto_inc: bool,
    pub unique: bool,
    pub required: bool,
}

/// Parsed adjacency declaration: ADJACENCY name ON ... WEIGHT w
#[derive(Debug, Clone, PartialEq)]
pub struct AdjacencySpec {
    pub name: String,
    pub kind: AdjacencySpecKind,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdjacencySpecKind {
    /// ON field = field
    Equality { field: String },
    /// ON field WITHIN radius
    Metric { field: String, radius: f64 },
    /// ON field ABOVE threshold
    Threshold { field: String, threshold: f64 },
    /// ON field_a TO field_b VIA transform_fn
    Transform {
        source_field: String,
        target_field: String,
        transform: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectCol {
    Name(String),
    Star,
    Agg(AggFunc, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Eq(String, Literal),
    Between(String, Literal, Literal),
    In(String, Vec<Literal>),
}

/// Filter condition for COVER WHERE / ON clauses.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterCondition {
    Eq(String, Literal),
    Neq(String, Literal),
    Gt(String, Literal),
    Gte(String, Literal),
    Lt(String, Literal),
    Lte(String, Literal),
    In(String, Vec<Literal>),
    NotIn(String, Vec<Literal>),
    Contains(String, String),
    StartsWith(String, String),
    EndsWith(String, String),
    Matches(String, String),
    Void(String),
    Defined(String),
    Between(String, Literal, Literal),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortSpec {
    pub field: String,
    pub desc: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureSpec {
    pub func: AggFunc,
    pub field: String,
    pub alias: Option<String>,
}

// ── Tokenizer ──

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Number(f64),
    Str(String),
    LParen,
    RParen,
    Comma,
    Eq,
    Neq, // != or <>
    Gt,  // >
    Gte, // >=
    Lt,  // <
    Lte, // <=
    Star,
    Dot,
    Colon, // :
    Semicolon,
    Plus,
    Minus,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            // Line comments: -- ...
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '!' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Neq);
                i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '>' => {
                tokens.push(Token::Neq);
                i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Lte);
                i += 2;
            }
            '<' => {
                tokens.push(Token::Lt);
                i += 1;
            }
            '>' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Gte);
                i += 2;
            }
            '>' => {
                tokens.push(Token::Gt);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '\'' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("Unterminated string literal".into());
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Str(s));
                i += 1;
            }
            '-' => {
                // Could be negative number or minus
                if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    let start = i;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '.' {
                        i += 1;
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let s: String = chars[start..i].iter().collect();
                    let n: f64 = s.parse().map_err(|_| format!("Invalid number: {s}"))?;
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Minus);
                    i += 1;
                }
            }
            '0'..='9' => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '.' {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                let n: f64 = s.parse().map_err(|_| format!("Invalid number: {s}"))?;
                tokens.push(Token::Number(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Word(word));
            }
            '$' => {
                // Parameter placeholder: $1, $2, etc.
                let start = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Word(word));
            }
            c => return Err(format!("Unexpected character: {c}")),
        }
    }
    Ok(tokens)
}

// ── Parser ──

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect_word(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(Token::Word(w)) => Ok(w),
            other => Err(format!("Expected identifier, got {other:?}")),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), String> {
        match self.advance() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw) => Ok(()),
            other => Err(format!("Expected '{kw}', got {other:?}")),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let t = self.advance();
        if t.as_ref() == Some(&expected) {
            Ok(())
        } else {
            Err(format!("Expected {expected:?}, got {t:?}"))
        }
    }

    fn is_keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn expect_usize(&mut self) -> Result<usize, String> {
        match self.advance() {
            Some(Token::Number(n)) if n >= 0.0 && n.fract() == 0.0 => Ok(n as usize),
            Some(Token::Word(w)) => w
                .parse()
                .map_err(|_| format!("Expected positive integer, got '{w}'")),
            other => Err(format!("Expected positive integer, got {other:?}")),
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek(), Some(Token::Semicolon))
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        match self.advance() {
            Some(Token::Number(n)) => {
                if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
                    Ok(Literal::Integer(n as i64))
                } else {
                    Ok(Literal::Float(n))
                }
            }
            Some(Token::Str(s)) => Ok(Literal::Text(s)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("true") => Ok(Literal::Bool(true)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("false") => Ok(Literal::Bool(false)),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("null") => Ok(Literal::Null),
            other => Err(format!("Expected literal, got {other:?}")),
        }
    }

    // ── Top-level dispatch ──

    fn parse(&mut self) -> Result<Statement, String> {
        let first = self.expect_word()?;
        match first.to_ascii_uppercase().as_str() {
            // SQL compat
            "CREATE" => self.parse_create_bundle(),
            "INSERT" => self.parse_sql_insert(),
            "SELECT" => self.parse_sql_select(),

            // GQL native
            "BUNDLE" => self.parse_bundle(),
            "SECTION" => self.parse_section(),
            "SECTIONS" => self.parse_sections(),
            "REDEFINE" => self.parse_redefine(),
            "RETRACT" => self.parse_retract(),
            "COVER" => self.parse_cover(),
            "INTEGRATE" => self.parse_integrate(),
            "PULLBACK" => self.parse_pullback(),
            "COLLAPSE" => {
                let name = self.expect_word()?;
                Ok(Statement::Collapse { bundle: name })
            }
            "EXPLAIN" => self.parse_explain(),
            "SHOW" => self.parse_show(),
            "DESCRIBE" => {
                let name = self.expect_word()?;
                let verbose = self.is_keyword("VERBOSE");
                if verbose {
                    self.advance();
                }
                Ok(Statement::Describe {
                    bundle: name,
                    verbose,
                })
            }
            "HEALTH" => {
                let name = self.expect_word()?;
                Ok(Statement::Health { bundle: name })
            }
            "EXISTS" => self.parse_exists(),
            "ATLAS" => self.parse_atlas(),

            // Analytics
            "CURVATURE" => self.parse_curvature(),
            "SPECTRAL" => self.parse_spectral(),
            "CONSISTENCY" => self.parse_consistency(),
            "COMPLETE" => self.parse_complete(),
            "PROPAGATE" => self.parse_propagate(),
            "SUGGEST_ADJACENCY" => self.parse_suggest_adjacency(),

            // v2.1: Access Control
            "WEAVE" => self.parse_weave(),
            "UNWEAVE" => self.parse_unweave(),
            "GRANT" => self.parse_grant(),
            "REVOKE" => self.parse_revoke(),
            "POLICY" => self.parse_policy(),
            "DROP" => self.parse_drop(),
            "AUDIT" => self.parse_audit(),

            // v2.1: Constraints
            "GAUGE" => self.parse_gauge(),

            // v2.1: Maintenance
            "COMPACT" => self.parse_compact(),
            "ANALYZE" => self.parse_analyze(),
            "VACUUM" => self.parse_vacuum(),
            "REBUILD" => self.parse_rebuild(),
            "CHECK" => self.parse_check(),
            "REPAIR" => {
                let name = self.expect_word()?;
                Ok(Statement::Repair { bundle: name })
            }
            "STORAGE" => {
                let name = self.expect_word()?;
                Ok(Statement::StorageInfo { bundle: name })
            }

            // v2.1: Session
            "SET" => self.parse_set(),
            "RESET" => self.parse_reset(),

            // v2.1: Data Movement
            "INGEST" => self.parse_ingest(),
            "TRANSPLANT" => self.parse_transplant(),
            "GENERATE" => self.parse_generate(),
            "FILL" => self.parse_fill(),

            // v2.1: Prepared Statements
            "PREPARE" => self.parse_prepare(),
            "EXECUTE" => self.parse_execute(),
            "DEALLOCATE" => self.parse_deallocate(),

            // v2.1: Backup / Restore
            "BACKUP" => self.parse_backup(),
            "RESTORE" => self.parse_restore(),
            "VERIFY" => self.parse_verify(),

            // v2.1: Comments
            "COMMENT" => self.parse_comment(),

            // v2.1: Recursive
            "ITERATE" => self.parse_iterate(),

            // v2.1: Triggers
            "BEFORE" | "AFTER" | "ON" => self.parse_trigger(&first),

            // Feature #6: Cache invalidation
            "INVALIDATE" => self.parse_invalidate_cache(),

            _ => Err(format!("Unknown statement: {first}")),
        }
    }

    // ── GQL: BUNDLE ──

    fn parse_bundle(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        // Optional opening paren (SQL-style) or keyword-style
        if matches!(self.peek(), Some(Token::LParen)) {
            // SQL-style: BUNDLE name (field TYPE ..., ...)
            return self.parse_bundle_fields_paren(name);
        }

        let mut base_fields = Vec::new();
        let mut fiber_fields = Vec::new();
        let mut indexed = Vec::new();

        // Keyword-style: BUNDLE name BASE (...) FIBER (...)
        if self.is_keyword("BASE") {
            self.advance();
            self.expect(Token::LParen)?;
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !base_fields.is_empty() {
                    self.expect(Token::Comma)?;
                }
                base_fields.push(self.parse_field_spec(&mut indexed)?);
            }
            self.expect(Token::RParen)?;
        }

        if self.is_keyword("FIBER") {
            self.advance();
            self.expect(Token::LParen)?;
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !fiber_fields.is_empty() {
                    self.expect(Token::Comma)?;
                }
                fiber_fields.push(self.parse_field_spec(&mut indexed)?);
            }
            self.expect(Token::RParen)?;
        }

        let encrypted = self.is_keyword("ENCRYPTED");
        if encrypted {
            self.advance();
        }

        // ADJACENCY clauses: ADJACENCY name ON field = field WEIGHT w
        let mut adjacencies = Vec::new();
        while self.is_keyword("ADJACENCY") {
            self.advance();
            adjacencies.push(self.parse_adjacency_spec()?);
        }

        Ok(Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
        })
    }

    /// Parse: name ON field = field WEIGHT w | name ON field WITHIN r WEIGHT w | name ON field ABOVE t WEIGHT w
    fn parse_adjacency_spec(&mut self) -> Result<AdjacencySpec, String> {
        let adj_name = self.expect_word()?;
        self.expect_keyword("ON")?;
        let field = self.expect_word()?;

        let kind = if matches!(self.peek(), Some(Token::Eq)) {
            // Equality: ON field = field
            self.advance(); // consume =
            let _rhs = self.expect_word()?; // consume the repeated field name
            AdjacencySpecKind::Equality { field }
        } else if self.is_keyword("WITHIN") {
            self.advance();
            match self.advance() {
                Some(Token::Number(r)) => AdjacencySpecKind::Metric { field, radius: r },
                other => {
                    return Err(format!(
                        "Expected radius number after WITHIN, got {other:?}"
                    ))
                }
            }
        } else if self.is_keyword("ABOVE") {
            self.advance();
            match self.advance() {
                Some(Token::Number(t)) => AdjacencySpecKind::Threshold {
                    field,
                    threshold: t,
                },
                other => {
                    return Err(format!(
                        "Expected threshold number after ABOVE, got {other:?}"
                    ))
                }
            }
        } else if self.is_keyword("TO") {
            // Transform: ON field_a TO field_b VIA fn_name
            self.advance(); // consume TO
            let target_field = self.expect_word()?;
            self.expect_keyword("VIA")?;
            let transform = self.expect_word()?;
            AdjacencySpecKind::Transform {
                source_field: field,
                target_field,
                transform,
            }
        } else {
            return Err(format!(
                "Expected =, WITHIN, ABOVE, or TO after ADJACENCY ON {field}"
            ));
        };

        self.expect_keyword("WEIGHT")?;
        let weight = match self.advance() {
            Some(Token::Number(w)) => w,
            other => return Err(format!("Expected weight number, got {other:?}")),
        };

        Ok(AdjacencySpec {
            name: adj_name,
            kind,
            weight,
        })
    }

    fn parse_field_spec(&mut self, indexed: &mut Vec<String>) -> Result<FieldSpec, String> {
        let name = self.expect_word()?;
        let ftype = self.expect_word()?;
        let mut range = None;
        let mut default = None;
        let mut auto_inc = false;
        let mut unique = false;
        let mut required = false;

        loop {
            if self.is_keyword("RANGE") {
                self.advance();
                // RANGE n or RANGE(n)
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    match self.advance() {
                        Some(Token::Number(n)) => range = Some(n),
                        other => return Err(format!("Expected range value, got {other:?}")),
                    }
                    self.expect(Token::RParen)?;
                } else {
                    match self.advance() {
                        Some(Token::Number(n)) => range = Some(n),
                        other => return Err(format!("Expected range value, got {other:?}")),
                    }
                }
            } else if self.is_keyword("DEFAULT") {
                self.advance();
                default = Some(self.parse_literal()?);
            } else if self.is_keyword("AUTO") {
                self.advance();
                auto_inc = true;
            } else if self.is_keyword("UNIQUE") {
                self.advance();
                unique = true;
            } else if self.is_keyword("REQUIRED") {
                self.advance();
                required = true;
            } else if self.is_keyword("INDEX") {
                self.advance();
                indexed.push(name.clone());
            } else {
                break;
            }
        }

        Ok(FieldSpec {
            name,
            ftype,
            range,
            default,
            auto_inc,
            unique,
            required,
        })
    }

    // ── GQL: SECTION (insert / point query) ──

    fn parse_section(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        // SECTION name AT k=v → point query
        if self.is_keyword("AT") {
            self.advance();
            let key = self.parse_kv_pairs()?;
            let mut project = None;
            if self.is_keyword("PROJECT") {
                self.advance();
                project = Some(self.parse_name_list()?);
            }
            return Ok(Statement::PointQuery {
                bundle: name,
                key,
                project,
            });
        }

        // SECTION name (...) [UPSERT] → insert
        self.expect(Token::LParen)?;
        let (columns, values) = self.parse_section_body()?;
        self.expect(Token::RParen)?;

        if self.is_keyword("UPSERT") {
            self.advance();
            return Ok(Statement::SectionUpsert {
                bundle: name,
                columns,
                values,
            });
        }

        Ok(Statement::Insert {
            bundle: name,
            columns,
            values,
        })
    }

    fn parse_section_body(&mut self) -> Result<(Vec<String>, Vec<Literal>), String> {
        let mut columns = Vec::new();
        let mut values = Vec::new();

        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !columns.is_empty() {
                self.expect(Token::Comma)?;
            }

            let col = self.expect_word()?;
            // Accept either : or = as separator
            if matches!(self.peek(), Some(Token::Colon)) || matches!(self.peek(), Some(Token::Eq)) {
                self.advance();
            } else {
                return Err(format!("Expected ':' or '=' after field name '{col}'"));
            }
            let val = self.parse_literal()?;
            columns.push(col);
            values.push(val);
        }

        Ok((columns, values))
    }

    // ── GQL: SECTIONS (batch insert) ──

    fn parse_sections(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        self.expect(Token::LParen)?;

        // Detect which of 3 patterns:
        // 1) Named: SECTIONS b (col: val, col: val, ...)
        // 2) Column-list + tuples: SECTIONS b (col, col, ...) (v, v, ...), (v, v, ...)
        // 3) Positional: SECTIONS b (v, v, v, ...)
        let named = self.pos + 1 < self.tokens.len()
            && matches!(self.tokens.get(self.pos), Some(Token::Word(_)))
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(Token::Colon) | Some(Token::Eq)
            );

        // Check for column-list pattern: Word followed by , or ) (not : or =)
        let column_list = !named
            && matches!(self.tokens.get(self.pos), Some(Token::Word(_)))
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(Token::Comma) | Some(Token::RParen)
            );

        if named {
            // Pattern 1: Named key-value pairs, single row
            let mut columns = Vec::new();
            let mut rows = Vec::new();
            let mut current_row = Vec::new();

            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !columns.is_empty() || !current_row.is_empty() {
                    self.expect(Token::Comma)?;
                }
                let col = self.expect_word()?;
                if matches!(self.peek(), Some(Token::Colon))
                    || matches!(self.peek(), Some(Token::Eq))
                {
                    self.advance();
                }
                let val = self.parse_literal()?;
                columns.push(col);
                current_row.push(val);
            }
            rows.push(current_row);
            self.expect(Token::RParen)?;

            Ok(Statement::BatchInsert {
                bundle: name,
                columns,
                rows,
            })
        } else if column_list {
            // Pattern 2: SECTIONS b (col1, col2, ...) (v1, v2, ...), (v1, v2, ...)
            let mut columns = Vec::new();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !columns.is_empty() {
                    self.expect(Token::Comma)?;
                }
                columns.push(self.expect_word()?);
            }
            self.expect(Token::RParen)?;

            // Now parse value tuples
            let mut rows = Vec::new();
            loop {
                if !rows.is_empty() {
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance(); // consume comma between tuples
                    } else {
                        break;
                    }
                }
                if !matches!(self.peek(), Some(Token::LParen)) {
                    break;
                }
                self.expect(Token::LParen)?;
                let mut row = Vec::new();
                loop {
                    if matches!(self.peek(), Some(Token::RParen)) {
                        break;
                    }
                    if !row.is_empty() {
                        self.expect(Token::Comma)?;
                    }
                    row.push(self.parse_literal()?);
                }
                self.expect(Token::RParen)?;
                rows.push(row);
            }

            Ok(Statement::BatchInsert {
                bundle: name,
                columns,
                rows,
            })
        } else {
            // Pattern 3: Positional values only, single row
            let mut all_values = Vec::new();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !all_values.is_empty() {
                    self.expect(Token::Comma)?;
                }
                all_values.push(self.parse_literal()?);
            }
            self.expect(Token::RParen)?;

            Ok(Statement::BatchInsert {
                bundle: name,
                columns: vec![],
                rows: vec![all_values],
            })
        }
    }

    // ── GQL: REDEFINE (update) ──

    fn parse_redefine(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        if self.is_keyword("AT") {
            // Point update: REDEFINE name AT k=v SET (...)
            self.advance();
            let key = self.parse_kv_pairs()?;
            self.expect_keyword("SET")?;
            self.expect(Token::LParen)?;
            let sets = self.parse_kv_pairs_inner()?;
            self.expect(Token::RParen)?;
            Ok(Statement::Redefine {
                bundle: name,
                key,
                sets,
            })
        } else if self.is_keyword("ON") || self.is_keyword("WHERE") {
            // Bulk update: REDEFINE name ON/WHERE conditions SET (...)
            let conditions = self.parse_filter_conditions()?;
            self.expect_keyword("SET")?;
            self.expect(Token::LParen)?;
            let sets = self.parse_kv_pairs_inner()?;
            self.expect(Token::RParen)?;
            Ok(Statement::BulkRedefine {
                bundle: name,
                conditions,
                sets,
            })
        } else {
            Err("REDEFINE requires AT or ON/WHERE clause".into())
        }
    }

    // ── GQL: RETRACT (delete) ──

    fn parse_retract(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        if self.is_keyword("AT") {
            self.advance();
            let key = self.parse_kv_pairs()?;
            Ok(Statement::Retract { bundle: name, key })
        } else if self.is_keyword("ON") || self.is_keyword("WHERE") {
            let conditions = self.parse_filter_conditions()?;
            Ok(Statement::BulkRetract {
                bundle: name,
                conditions,
            })
        } else {
            Err("RETRACT requires AT or ON/WHERE clause".into())
        }
    }

    // ── GQL: COVER (range/filtered query) ──

    fn parse_cover(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;

        let mut on_conditions = Vec::new();
        let mut where_conditions = Vec::new();
        let mut or_groups = Vec::new();
        let mut distinct_field = None;
        let mut project = None;
        let mut rank_by = None;
        let mut first = None;
        let mut skip = None;
        let mut all = false;

        // Parse optional clauses in any order
        loop {
            if self.at_end() {
                break;
            }

            if self.is_keyword("ALL") {
                self.advance();
                all = true;
            } else if self.is_keyword("ON") {
                self.advance();
                let conds = self.parse_filter_condition_list()?;
                on_conditions.extend(conds);
            } else if self.is_keyword("WHERE") {
                self.advance();
                let conds = self.parse_filter_condition_list()?;
                where_conditions.extend(conds);
            } else if self.is_keyword("OR") {
                self.advance();
                // Parse OR group
                let conds = self.parse_filter_condition_list()?;
                or_groups.push(conds);
            } else if self.is_keyword("DISTINCT") {
                self.advance();
                distinct_field = Some(self.expect_word()?);
            } else if self.is_keyword("PROJECT") {
                self.advance();
                project = Some(self.parse_name_list()?);
            } else if self.is_keyword("RANK") {
                self.advance();
                self.expect_keyword("BY")?;
                rank_by = Some(self.parse_sort_specs()?);
            } else if self.is_keyword("FIRST") {
                self.advance();
                first = Some(self.parse_usize()?);
            } else if self.is_keyword("SKIP") {
                self.advance();
                skip = Some(self.parse_usize()?);
            } else {
                break;
            }
        }

        Ok(Statement::Cover {
            bundle: name,
            on_conditions,
            where_conditions,
            or_groups,
            distinct_field,
            project,
            rank_by,
            first,
            skip,
            all,
        })
    }

    // ── GQL: INTEGRATE (aggregation) ──

    fn parse_integrate(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut over = None;
        let mut measures = Vec::new();

        if self.is_keyword("OVER") {
            self.advance();
            over = Some(self.expect_word()?);
        }

        if self.is_keyword("MEASURE") {
            self.advance();
            loop {
                let func_name = self.expect_word()?;
                let func = match func_name.to_ascii_uppercase().as_str() {
                    "COUNT" => AggFunc::Count,
                    "SUM" => AggFunc::Sum,
                    "AVG" => AggFunc::Avg,
                    "MIN" => AggFunc::Min,
                    "MAX" => AggFunc::Max,
                    _ => return Err(format!("Unknown aggregate: {func_name}")),
                };
                self.expect(Token::LParen)?;
                let field = if matches!(self.peek(), Some(Token::Star)) {
                    self.advance();
                    "*".to_string()
                } else {
                    self.expect_word()?
                };
                self.expect(Token::RParen)?;

                let alias = if self.is_keyword("AS") {
                    self.advance();
                    Some(self.expect_word()?)
                } else {
                    None
                };

                measures.push(MeasureSpec { func, field, alias });

                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance(); // consume comma
            }
        }

        Ok(Statement::Integrate {
            bundle: name,
            over,
            measures,
        })
    }

    // ── GQL: PULLBACK (join) ──

    fn parse_pullback(&mut self) -> Result<Statement, String> {
        let left = self.expect_word()?;
        self.expect_keyword("ALONG")?;
        let along = self.expect_word()?;
        self.expect_keyword("ONTO")?;
        let right = self.expect_word()?;

        let right_field = if self.is_keyword("ALONG") {
            self.advance();
            Some(self.expect_word()?)
        } else {
            None
        };

        let preserve_left = if self.is_keyword("PRESERVE") {
            self.advance();
            self.expect_keyword("LEFT")?;
            true
        } else {
            false
        };

        Ok(Statement::Pullback {
            left,
            along,
            right,
            right_field,
            preserve_left,
        })
    }

    // ── GQL: CURVATURE / SPECTRAL / CONSISTENCY ──

    fn parse_curvature(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut fields = Vec::new();
        let mut by_field = None;

        if self.is_keyword("ON") {
            self.advance();
            loop {
                fields.push(self.expect_word()?);
                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance();
            }
        }

        if self.is_keyword("BY") {
            self.advance();
            by_field = Some(self.expect_word()?);
        }

        Ok(Statement::Curvature {
            bundle: name,
            fields,
            by_field,
        })
    }

    fn parse_spectral(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let full = if self.is_keyword("FULL") {
            self.advance();
            true
        } else {
            false
        };
        Ok(Statement::Spectral { bundle: name, full })
    }

    fn parse_consistency(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let repair = if self.is_keyword("REPAIR") {
            self.advance();
            true
        } else {
            false
        };
        Ok(Statement::Consistency {
            bundle: name,
            repair,
        })
    }

    // ── GQL: COMPLETE / PROPAGATE ──

    fn parse_complete(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        let mut where_conditions = Vec::new();
        let mut method = None;
        let mut min_confidence = None;
        let mut with_provenance = false;
        let mut with_constraint_graph = false;

        if self.is_keyword("WHERE") {
            self.advance();
            where_conditions = self.parse_filter_conditions()?;
        }
        if self.is_keyword("METHOD") {
            self.advance();
            method = Some(self.expect_word()?);
        }
        if self.is_keyword("MIN_CONFIDENCE") {
            self.advance();
            match self.advance() {
                Some(Token::Number(n)) => min_confidence = Some(n),
                other => return Err(format!("Expected confidence number, got {other:?}")),
            }
        }
        if self.is_keyword("WITH") {
            self.advance();
            loop {
                let kw = self.expect_word()?;
                match kw.to_ascii_uppercase().as_str() {
                    "PROVENANCE" => with_provenance = true,
                    "CONSTRAINT_GRAPH" => with_constraint_graph = true,
                    _ => return Err(format!("Unknown WITH option: {kw}")),
                }
                if !matches!(self.peek(), Some(Token::Comma)) {
                    break;
                }
                self.advance();
            }
        }

        Ok(Statement::Complete {
            bundle,
            where_conditions,
            method,
            min_confidence,
            with_provenance,
            with_constraint_graph,
        })
    }

    fn parse_propagate(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("ASSUMING")?;
        let assumptions = self.parse_kv_pairs()?;
        // Optional: SHOW NEWLY_DETERMINED (ignored — always returned)
        if self.is_keyword("SHOW") {
            self.advance();
            if self.is_keyword("NEWLY_DETERMINED") {
                self.advance();
            }
        }
        Ok(Statement::Propagate {
            bundle,
            assumptions,
        })
    }

    // ── GQL: SUGGEST_ADJACENCY ──

    fn parse_suggest_adjacency(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;

        let mut fields = Vec::new();
        let mut sample_size = 10_000_usize;
        let mut candidates = 5_usize;

        loop {
            if self.is_keyword("FIELDS") {
                self.advance();
                // Parse comma-separated field list
                loop {
                    fields.push(self.expect_word()?);
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            } else if self.is_keyword("SAMPLE_SIZE") {
                self.advance();
                sample_size = self.expect_usize()?;
            } else if self.is_keyword("CANDIDATES") {
                self.advance();
                candidates = self.expect_usize()?;
            } else if self.is_keyword("MINIMIZING") {
                self.advance();
                self.expect_keyword("h1")?; // only h1 for now
            } else {
                break;
            }
        }

        Ok(Statement::SuggestAdjacency {
            bundle,
            fields,
            sample_size,
            candidates,
        })
    }

    // ── GQL: EXPLAIN ──

    fn parse_explain(&mut self) -> Result<Statement, String> {
        let inner = self.parse()?;
        Ok(Statement::Explain {
            inner: Box::new(inner),
        })
    }

    // ── GQL: EXISTS ──

    fn parse_exists(&mut self) -> Result<Statement, String> {
        self.expect_keyword("SECTION")?;
        let name = self.expect_word()?;
        self.expect_keyword("AT")?;
        let key = self.parse_kv_pairs()?;
        Ok(Statement::ExistsSection { bundle: name, key })
    }

    // ── GQL: ATLAS (transactions) ──

    fn parse_atlas(&mut self) -> Result<Statement, String> {
        let action = self.expect_word()?;
        match action.to_ascii_uppercase().as_str() {
            "BEGIN" => Ok(Statement::AtlasBegin),
            "COMMIT" => Ok(Statement::AtlasCommit),
            "ROLLBACK" => Ok(Statement::AtlasRollback),
            _ => Err(format!("Unknown ATLAS action: {action}")),
        }
    }

    // ── SQL compat: CREATE BUNDLE ──

    fn parse_create_bundle(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BUNDLE")?;
        let name = self.expect_word()?;
        self.parse_bundle_fields_paren(name)
    }

    fn parse_bundle_fields_paren(&mut self, name: String) -> Result<Statement, String> {
        self.expect(Token::LParen)?;

        let mut base_fields = Vec::new();
        let mut fiber_fields = Vec::new();
        let mut indexed = Vec::new();

        loop {
            if self.is_keyword("BASE") || self.is_keyword("FIBER") || self.is_keyword("INDEX") {
                break;
            }
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !base_fields.is_empty() || !fiber_fields.is_empty() {
                self.expect(Token::Comma)?;
            }

            let fname = self.expect_word()?;
            let ftype = self.expect_word()?;
            let mut range = None;

            if self.is_keyword("RANGE") {
                self.advance();
                self.expect(Token::LParen)?;
                match self.advance() {
                    Some(Token::Number(n)) => range = Some(n),
                    other => return Err(format!("Expected range value, got {other:?}")),
                }
                self.expect(Token::RParen)?;
            }

            let spec = FieldSpec {
                name: fname,
                ftype,
                range,
                default: None,
                auto_inc: false,
                unique: false,
                required: false,
            };

            if self.is_keyword("BASE") {
                self.advance();
                base_fields.push(spec);
            } else if self.is_keyword("FIBER") {
                self.advance();
                fiber_fields.push(spec);
            } else if base_fields.is_empty() {
                base_fields.push(spec);
            } else {
                fiber_fields.push(spec);
            }

            if self.is_keyword("INDEX") {
                self.advance();
                let last = if fiber_fields.is_empty() {
                    base_fields.last().unwrap()
                } else {
                    fiber_fields.last().unwrap()
                };
                indexed.push(last.name.clone());
            }
        }

        self.expect(Token::RParen)?;
        let encrypted = self.is_keyword("ENCRYPTED");
        if encrypted {
            self.advance();
        }

        // ADJACENCY clauses after SQL-style CREATE BUNDLE are also supported
        let mut adjacencies = Vec::new();
        while self.is_keyword("ADJACENCY") {
            self.advance();
            adjacencies.push(self.parse_adjacency_spec()?);
        }

        Ok(Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
        })
    }

    fn parse_sql_insert(&mut self) -> Result<Statement, String> {
        self.expect_keyword("INTO")?;
        let bundle = self.expect_word()?;

        let mut columns = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                columns.push(self.expect_word()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
        }

        self.expect_keyword("VALUES")?;
        self.expect(Token::LParen)?;

        let mut values = Vec::new();
        loop {
            values.push(self.parse_literal()?);
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            self.expect(Token::Comma)?;
        }
        self.expect(Token::RParen)?;

        Ok(Statement::Insert {
            bundle,
            columns,
            values,
        })
    }

    // ── SQL compat: SELECT ──

    fn parse_sql_select(&mut self) -> Result<Statement, String> {
        let mut columns = Vec::new();
        loop {
            if self.is_keyword("FROM") {
                break;
            }
            if !columns.is_empty() {
                self.expect(Token::Comma)?;
            }
            columns.push(self.parse_select_col()?);
        }

        self.expect_keyword("FROM")?;
        let bundle = self.expect_word()?;

        if self.is_keyword("JOIN") {
            self.advance();
            let right = self.expect_word()?;
            self.expect_keyword("ON")?;
            let on_field = self.expect_word()?;
            return Ok(Statement::Join {
                left: bundle,
                right,
                on_field,
                columns,
            });
        }

        let mut condition = None;
        let mut group_by = None;

        if self.is_keyword("WHERE") {
            self.advance();
            condition = Some(self.parse_sql_condition()?);
        }

        if self.is_keyword("GROUP") {
            self.advance();
            self.expect_keyword("BY")?;
            group_by = Some(self.expect_word()?);
        }

        Ok(Statement::Select {
            bundle,
            columns,
            condition,
            group_by,
        })
    }

    fn parse_select_col(&mut self) -> Result<SelectCol, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            return Ok(SelectCol::Star);
        }

        let word = self.expect_word()?;
        let upper = word.to_ascii_uppercase();

        let agg = match upper.as_str() {
            "COUNT" => Some(AggFunc::Count),
            "SUM" => Some(AggFunc::Sum),
            "AVG" => Some(AggFunc::Avg),
            "MIN" => Some(AggFunc::Min),
            "MAX" => Some(AggFunc::Max),
            _ => None,
        };

        if let Some(func) = agg {
            if matches!(self.peek(), Some(Token::LParen)) {
                self.advance();
                let field = self.expect_word()?;
                self.expect(Token::RParen)?;
                return Ok(SelectCol::Agg(func, field));
            }
        }

        Ok(SelectCol::Name(word))
    }

    fn parse_sql_condition(&mut self) -> Result<Condition, String> {
        let field = self.expect_word()?;

        if self.is_keyword("BETWEEN") {
            self.advance();
            let lo = self.parse_literal()?;
            self.expect_keyword("AND")?;
            let hi = self.parse_literal()?;
            return Ok(Condition::Between(field, lo, hi));
        }

        if self.is_keyword("IN") {
            self.advance();
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(Condition::In(field, vals));
        }

        self.expect(Token::Eq)?;
        let val = self.parse_literal()?;
        Ok(Condition::Eq(field, val))
    }

    // ── Helper: parse key=value pairs ──

    fn parse_kv_pairs(&mut self) -> Result<Vec<(String, Literal)>, String> {
        let mut pairs = Vec::new();
        loop {
            if self.at_end() {
                break;
            }
            // Stop at known clause keywords
            if self.is_keyword("SET")
                || self.is_keyword("PROJECT")
                || self.is_keyword("RANK")
                || self.is_keyword("FIRST")
                || self.is_keyword("SKIP")
                || self.is_keyword("ON")
                || self.is_keyword("WHERE")
                || self.is_keyword("MEASURE")
                || self.is_keyword("OVER")
                || self.is_keyword("UPSERT")
            {
                break;
            }
            if !pairs.is_empty() {
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
            let key = self.expect_word()?;
            // Accept = or :
            if matches!(self.peek(), Some(Token::Eq)) || matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
            } else {
                return Err(format!("Expected '=' or ':' after '{key}'"));
            }
            let val = self.parse_literal()?;
            pairs.push((key, val));
        }
        Ok(pairs)
    }

    fn parse_kv_pairs_inner(&mut self) -> Result<Vec<(String, Literal)>, String> {
        let mut pairs = Vec::new();
        loop {
            if matches!(self.peek(), Some(Token::RParen)) {
                break;
            }
            if !pairs.is_empty() {
                self.expect(Token::Comma)?;
            }
            let key = self.expect_word()?;
            if matches!(self.peek(), Some(Token::Eq)) || matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
            } else {
                return Err(format!("Expected '=' or ':' after '{key}'"));
            }
            let val = self.parse_literal()?;
            pairs.push((key, val));
        }
        Ok(pairs)
    }

    // ── Helper: filter conditions ──

    fn parse_filter_conditions(&mut self) -> Result<Vec<FilterCondition>, String> {
        // Consume ON or WHERE keyword
        if self.is_keyword("ON") || self.is_keyword("WHERE") {
            self.advance();
        }
        self.parse_filter_condition_list()
    }

    fn parse_filter_condition_list(&mut self) -> Result<Vec<FilterCondition>, String> {
        let mut conditions = Vec::new();
        loop {
            conditions.push(self.parse_single_filter()?);
            if self.is_keyword("AND") {
                self.advance();
            } else {
                break;
            }
        }
        Ok(conditions)
    }

    fn parse_single_filter(&mut self) -> Result<FilterCondition, String> {
        let field = self.expect_word()?;

        // Check for VOID / DEFINED
        if field.eq_ignore_ascii_case("VOID") || field.eq_ignore_ascii_case("DEFINED") {
            return Err("VOID/DEFINED must follow a field name".into());
        }

        // field VOID / field DEFINED
        if self.is_keyword("VOID") {
            self.advance();
            return Ok(FilterCondition::Void(field));
        }
        if self.is_keyword("DEFINED") {
            self.advance();
            return Ok(FilterCondition::Defined(field));
        }

        // field MATCHES 'pattern'
        if self.is_keyword("MATCHES") {
            self.advance();
            let pattern = match self.advance() {
                Some(Token::Str(s)) => s,
                other => return Err(format!("Expected string pattern, got {other:?}")),
            };
            return Ok(FilterCondition::Matches(field, pattern));
        }

        // field CONTAINS 'text'
        if self.is_keyword("CONTAINS") {
            self.advance();
            let text = match self.advance() {
                Some(Token::Str(s)) => s,
                other => return Err(format!("Expected string, got {other:?}")),
            };
            return Ok(FilterCondition::Contains(field, text));
        }

        // field BETWEEN lo AND hi
        if self.is_keyword("BETWEEN") {
            self.advance();
            let lo = self.parse_literal()?;
            self.expect_keyword("AND")?;
            let hi = self.parse_literal()?;
            return Ok(FilterCondition::Between(field, lo, hi));
        }

        // field IN (v1, v2, ...)
        if self.is_keyword("IN") {
            self.advance();
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(FilterCondition::In(field, vals));
        }

        // field NOT IN (v1, v2, ...)
        if self.is_keyword("NOT") {
            self.advance();
            self.expect_keyword("IN")?;
            self.expect(Token::LParen)?;
            let mut vals = Vec::new();
            loop {
                vals.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                self.expect(Token::Comma)?;
            }
            self.expect(Token::RParen)?;
            return Ok(FilterCondition::NotIn(field, vals));
        }

        // Comparison operators
        match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Eq(field, val))
            }
            Some(Token::Neq) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Neq(field, val))
            }
            Some(Token::Gt) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Gt(field, val))
            }
            Some(Token::Gte) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Gte(field, val))
            }
            Some(Token::Lt) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Lt(field, val))
            }
            Some(Token::Lte) => {
                self.advance();
                let val = self.parse_literal()?;
                Ok(FilterCondition::Lte(field, val))
            }
            other => Err(format!(
                "Expected comparison operator after '{field}', got {other:?}"
            )),
        }
    }

    // ── Helper: sort specs ──

    fn parse_sort_specs(&mut self) -> Result<Vec<SortSpec>, String> {
        let mut specs = Vec::new();
        loop {
            let field = self.expect_word()?;
            let desc = if self.is_keyword("DESC") {
                self.advance();
                true
            } else {
                if self.is_keyword("ASC") {
                    self.advance();
                }
                false
            };
            specs.push(SortSpec { field, desc });
            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }
            self.advance();
        }
        Ok(specs)
    }

    // ── Helper: name list ──

    fn parse_name_list(&mut self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }
                if !names.is_empty() {
                    self.expect(Token::Comma)?;
                }
                names.push(self.expect_word()?);
            }
            self.expect(Token::RParen)?;
        } else {
            names.push(self.expect_word()?);
        }
        Ok(names)
    }

    fn parse_usize(&mut self) -> Result<usize, String> {
        match self.advance() {
            Some(Token::Number(n)) if n >= 0.0 => Ok(n as usize),
            other => Err(format!("Expected positive integer, got {other:?}")),
        }
    }

    // ── v2.1: SHOW (extended) ──

    fn parse_show(&mut self) -> Result<Statement, String> {
        let what = self.expect_word()?;
        match what.to_ascii_uppercase().as_str() {
            "BUNDLES" => {
                let _verbose = self.is_keyword("VERBOSE");
                if _verbose {
                    self.advance();
                }
                Ok(Statement::ShowBundles)
            }
            "ROLES" => Ok(Statement::ShowRoles),
            "PREPARED" => Ok(Statement::ShowPrepared),
            "BACKUPS" => Ok(Statement::ShowBackups),
            "SETTINGS" => Ok(Statement::ShowSettings),
            "SESSION" => Ok(Statement::ShowSession),
            "CURRENT" => {
                self.expect_keyword("ROLE")?;
                Ok(Statement::ShowCurrentRole)
            }
            "FIELDS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowFields { bundle })
            }
            "INDEXES" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowIndexes { bundle })
            }
            "CONSTRAINTS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowConstraints { bundle })
            }
            "MORPHISMS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowMorphisms { bundle })
            }
            "TRIGGERS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowTriggers { bundle })
            }
            "POLICIES" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowPolicies { bundle })
            }
            "STATISTICS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowStatistics { bundle })
            }
            "GEOMETRY" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowGeometry { bundle })
            }
            "COMMENTS" => {
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::ShowComments { bundle })
            }
            _ => Err(format!("Unknown SHOW target: {what}")),
        }
    }

    // ── v2.1: Access Control ──

    fn parse_weave(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ROLE")?;
        let name = self.expect_word()?;
        let mut password = None;
        let mut inherits = None;
        let mut superweave = false;
        while !self.at_end() {
            if self.is_keyword("PASSWORD") {
                self.advance();
                match self.advance() {
                    Some(Token::Str(s)) => password = Some(s),
                    _ => return Err("Expected password string".into()),
                }
            } else if self.is_keyword("INHERITS") {
                self.advance();
                inherits = Some(self.expect_word()?);
            } else if self.is_keyword("SUPERWEAVE") {
                self.advance();
                superweave = true;
            } else {
                break;
            }
        }
        Ok(Statement::WeaveRole {
            name,
            password,
            inherits,
            superweave,
        })
    }

    fn parse_unweave(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ROLE")?;
        let name = self.expect_word()?;
        Ok(Statement::UnweaveRole { name })
    }

    fn parse_grant(&mut self) -> Result<Statement, String> {
        let mut operations = vec![self.expect_word()?];
        while self.is_keyword(",") || matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("TO")?;
        let role = self.expect_word()?;
        Ok(Statement::Grant {
            operations,
            bundle,
            role,
        })
    }

    fn parse_revoke(&mut self) -> Result<Statement, String> {
        let mut operations = vec![self.expect_word()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let role = self.expect_word()?;
        Ok(Statement::Revoke {
            operations,
            bundle,
            role,
        })
    }

    fn parse_policy(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("ON")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FOR")?;
        let mut operations = vec![self.expect_word()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            operations.push(self.expect_word()?);
        }
        self.expect_keyword("RESTRICT")?;
        self.expect_keyword("TO")?;
        // Capture the rest as the restrict query string
        let mut restrict_parts = Vec::new();
        let mut depth = 0i32;
        while !self.at_end() {
            if self.is_keyword("TO") && depth == 0 {
                break;
            }
            match self.peek() {
                Some(Token::LParen) => {
                    depth += 1;
                    restrict_parts.push("(".to_string());
                    self.advance();
                }
                Some(Token::RParen) => {
                    depth -= 1;
                    restrict_parts.push(")".to_string());
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Some(Token::Word(w)) => {
                    restrict_parts.push(w.clone());
                    self.advance();
                }
                Some(Token::Str(s)) => {
                    restrict_parts.push(format!("'{s}'"));
                    self.advance();
                }
                Some(Token::Number(n)) => {
                    restrict_parts.push(n.to_string());
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        let restrict_query = restrict_parts.join(" ");
        self.expect_keyword("TO")?;
        let role = self.expect_word()?;
        Ok(Statement::CreatePolicy {
            name,
            bundle,
            operations,
            restrict_query,
            role,
        })
    }

    fn parse_drop(&mut self) -> Result<Statement, String> {
        let what = self.expect_word()?;
        match what.to_ascii_uppercase().as_str() {
            "BUNDLE" => {
                let name = self.expect_word()?;
                Ok(Statement::Collapse { bundle: name })
            }
            "POLICY" => {
                let name = self.expect_word()?;
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::DropPolicy { name, bundle })
            }
            "TRIGGER" => {
                let name = self.expect_word()?;
                self.expect_keyword("ON")?;
                let bundle = self.expect_word()?;
                Ok(Statement::DropTrigger { name, bundle })
            }
            _ => Err(format!("Unknown DROP target: {what}")),
        }
    }

    fn parse_audit(&mut self) -> Result<Statement, String> {
        // AUDIT SHOW bundle ... or AUDIT bundle ON/OFF
        let next = self.expect_word()?;
        if next.eq_ignore_ascii_case("SHOW") {
            let bundle = self.expect_word()?;
            let mut since = None;
            let mut role = None;
            while !self.at_end() {
                if self.is_keyword("SINCE") {
                    self.advance();
                    match self.advance() {
                        Some(Token::Str(s)) => since = Some(s),
                        Some(Token::Number(n)) => since = Some(n.to_string()),
                        _ => return Err("Expected date after SINCE".into()),
                    }
                } else if self.is_keyword("ROLE") {
                    self.advance();
                    role = Some(self.expect_word()?);
                } else {
                    break;
                }
            }
            Ok(Statement::AuditShow {
                bundle,
                since,
                role,
            })
        } else {
            let bundle = next;
            let mode = self.expect_word()?;
            if mode.eq_ignore_ascii_case("OFF") {
                Ok(Statement::AuditOff { bundle })
            } else {
                // ON with optional operations list
                let mut operations = Vec::new();
                while !self.at_end() {
                    if self.is_keyword("SECTION")
                        || self.is_keyword("REDEFINE")
                        || self.is_keyword("RETRACT")
                    {
                        if let Some(Token::Word(w)) = self.advance() {
                            operations.push(w.to_ascii_uppercase());
                        }
                    } else {
                        break;
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                Ok(Statement::AuditOn { bundle, operations })
            }
        }
    }

    // ── v2.1: Constraints ──

    fn parse_gauge(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let action = self.expect_word()?;
        match action.to_ascii_uppercase().as_str() {
            "CONSTRAIN" => {
                // Capture everything in parens as constraint text
                self.expect(Token::LParen)?;
                let mut constraints = Vec::new();
                let mut current = String::new();
                let mut depth = 1i32;
                loop {
                    match self.advance() {
                        Some(Token::LParen) => {
                            depth += 1;
                            current.push('(');
                        }
                        Some(Token::RParen) => {
                            depth -= 1;
                            if depth == 0 {
                                if !current.trim().is_empty() {
                                    constraints.push(current.trim().to_string());
                                }
                                break;
                            }
                            current.push(')');
                        }
                        Some(Token::Comma) if depth == 1 => {
                            if !current.trim().is_empty() {
                                constraints.push(current.trim().to_string());
                            }
                            current = String::new();
                        }
                        Some(Token::Word(w)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(&w);
                        }
                        Some(Token::Number(n)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(&n.to_string());
                        }
                        Some(Token::Str(s)) => {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push('\'');
                            current.push_str(&s);
                            current.push('\'');
                        }
                        Some(Token::Eq) => current.push('='),
                        Some(Token::Gt) => current.push('>'),
                        Some(Token::Lt) => current.push('<'),
                        Some(Token::Gte) => current.push_str(">="),
                        Some(Token::Lte) => current.push_str("<="),
                        Some(Token::Neq) => current.push_str("!="),
                        Some(Token::Star) => current.push('*'),
                        Some(Token::Plus) => current.push('+'),
                        Some(Token::Minus) => current.push('-'),
                        None => return Err("Unexpected end in GAUGE CONSTRAIN".into()),
                        _ => {}
                    }
                }
                Ok(Statement::GaugeConstrain {
                    bundle,
                    constraints,
                })
            }
            "UNCONSTRAIN" => {
                let constraint_name = self.expect_word()?;
                Ok(Statement::GaugeUnconstrain {
                    bundle,
                    constraint_name,
                })
            }
            _ => Err(format!("Expected CONSTRAIN or UNCONSTRAIN, got {action}")),
        }
    }

    // ── v2.1: Maintenance ──

    fn parse_compact(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let analyze = self.is_keyword("ANALYZE");
        if analyze {
            self.advance();
        }
        Ok(Statement::Compact { bundle, analyze })
    }

    fn parse_analyze(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let mut field = None;
        let mut full = false;
        if self.is_keyword("ON") {
            self.advance();
            field = Some(self.expect_word()?);
        } else if self.is_keyword("FULL") {
            self.advance();
            full = true;
        }
        Ok(Statement::Analyze {
            bundle,
            field,
            full,
        })
    }

    fn parse_vacuum(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        let full = self.is_keyword("FULL");
        if full {
            self.advance();
        }
        Ok(Statement::Vacuum { bundle, full })
    }

    fn parse_rebuild(&mut self) -> Result<Statement, String> {
        self.expect_keyword("INDEX")?;
        let bundle = self.expect_word()?;
        let mut field = None;
        if self.is_keyword("ON") {
            self.advance();
            field = Some(self.expect_word()?);
        }
        Ok(Statement::RebuildIndex { bundle, field })
    }

    fn parse_check(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        Ok(Statement::CheckIntegrity { bundle })
    }

    // ── v2.1: Session ──

    fn parse_set(&mut self) -> Result<Statement, String> {
        let key = self.expect_word()?;
        let value = self.parse_literal()?;
        Ok(Statement::Set { key, value })
    }

    fn parse_reset(&mut self) -> Result<Statement, String> {
        if self.is_keyword("ALL") {
            self.advance();
            Ok(Statement::Reset { key: None })
        } else {
            let key = self.expect_word()?;
            Ok(Statement::Reset { key: Some(key) })
        }
    }

    // ── v2.1: Data Movement ──

    fn parse_ingest(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let source = match self.advance() {
            Some(Token::Str(s)) => s,
            Some(Token::Word(w)) => w, // STDIN
            other => return Err(format!("Expected source path, got {other:?}")),
        };
        self.expect_keyword("FORMAT")?;
        let format = self.expect_word()?;
        Ok(Statement::Ingest {
            bundle,
            source,
            format,
        })
    }

    fn parse_transplant(&mut self) -> Result<Statement, String> {
        let source = self.expect_word()?;
        self.expect_keyword("INTO")?;
        let target = self.expect_word()?;
        let mut conditions = Vec::new();
        let mut retract_source = false;
        if self.is_keyword("WHERE") {
            self.advance();
            conditions = self.parse_filter_condition_list()?;
        }
        if self.is_keyword("RETRACT") {
            self.advance();
            self.expect_keyword("SOURCE")?;
            retract_source = true;
        }
        Ok(Statement::Transplant {
            source,
            target,
            conditions,
            retract_source,
        })
    }

    fn parse_generate(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BASE")?;
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let field = self.expect_word()?;
        self.expect(Token::Eq)?;
        let from_val = self.parse_literal()?;
        self.expect_keyword("TO")?;
        // skip "field=" again
        let _field2 = self.expect_word()?;
        self.expect(Token::Eq)?;
        let to_val = self.parse_literal()?;
        self.expect_keyword("STEP")?;
        let step = self.parse_literal()?;
        Ok(Statement::GenerateBase {
            bundle,
            field,
            from_val,
            to_val,
            step,
        })
    }

    fn parse_fill(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("ON")?;
        let field = self.expect_word()?;
        self.expect_keyword("USING")?;
        let method = self.expect_word()?;
        // Optionally consume a qualifier like LINEAR
        let method = if self.is_keyword("LINEAR") || self.is_keyword("TRANSPORT") {
            let extra = self.expect_word()?;
            format!("{method} {extra}")
        } else {
            method
        };
        Ok(Statement::Fill {
            bundle,
            field,
            method,
        })
    }

    // ── v2.1: Prepared Statements ──

    fn parse_prepare(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        self.expect_keyword("AS")?;
        // Capture the rest of the tokens as the body string
        let mut parts = Vec::new();
        while !self.at_end() {
            match self.advance() {
                Some(Token::Word(w)) => parts.push(w),
                Some(Token::Number(n)) => parts.push(n.to_string()),
                Some(Token::Str(s)) => parts.push(format!("'{s}'")),
                Some(Token::LParen) => parts.push("(".into()),
                Some(Token::RParen) => parts.push(")".into()),
                Some(Token::Comma) => parts.push(",".into()),
                Some(Token::Eq) => parts.push("=".into()),
                Some(Token::Gt) => parts.push(">".into()),
                Some(Token::Lt) => parts.push("<".into()),
                Some(Token::Gte) => parts.push(">=".into()),
                Some(Token::Lte) => parts.push("<=".into()),
                Some(Token::Neq) => parts.push("!=".into()),
                Some(Token::Star) => parts.push("*".into()),
                Some(Token::Colon) => parts.push(":".into()),
                Some(Token::Plus) => parts.push("+".into()),
                Some(Token::Minus) => parts.push("-".into()),
                Some(Token::Dot) => parts.push(".".into()),
                _ => break,
            }
        }
        let body = parts.join(" ");
        Ok(Statement::Prepare { name, body })
    }

    fn parse_execute(&mut self) -> Result<Statement, String> {
        let name = self.expect_word()?;
        let mut params = Vec::new();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    self.advance();
                    break;
                }
                params.push(self.parse_literal()?);
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
            }
        }
        Ok(Statement::Execute { name, params })
    }

    fn parse_deallocate(&mut self) -> Result<Statement, String> {
        if self.is_keyword("ALL") {
            self.advance();
            Ok(Statement::Deallocate { name: None })
        } else {
            let name = self.expect_word()?;
            Ok(Statement::Deallocate { name: Some(name) })
        }
    }

    // ── v2.1: Backup / Restore ──

    fn parse_backup(&mut self) -> Result<Statement, String> {
        let first = self.expect_word()?;
        let (bundle, all) = if first.eq_ignore_ascii_case("ALL") {
            (None, true)
        } else {
            (Some(first), false)
        };
        self.expect_keyword("TO")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        let mut compress = false;
        let mut incremental_since = None;
        while !self.at_end() {
            if self.is_keyword("COMPRESS") {
                self.advance();
                compress = true;
            } else if self.is_keyword("INCREMENTAL") {
                self.advance();
                self.expect_keyword("SINCE")?;
                match self.advance() {
                    Some(Token::Str(s)) => incremental_since = Some(s),
                    _ => return Err("Expected date string after SINCE".into()),
                }
            } else {
                break;
            }
        }
        let bundle_name = if all { None } else { bundle };
        Ok(Statement::Backup {
            bundle: bundle_name,
            path,
            compress,
            incremental_since,
        })
    }

    fn parse_restore(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("FROM")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        let mut snapshot = None;
        let mut rename = None;
        while !self.at_end() {
            if self.is_keyword("AT") {
                self.advance();
                self.expect_keyword("SNAPSHOT")?;
                match self.advance() {
                    Some(Token::Str(s)) => snapshot = Some(s),
                    _ => return Err("Expected snapshot name".into()),
                }
            } else if self.is_keyword("AS") {
                self.advance();
                rename = Some(self.expect_word()?);
            } else {
                break;
            }
        }
        Ok(Statement::Restore {
            bundle,
            path,
            snapshot,
            rename,
        })
    }

    fn parse_verify(&mut self) -> Result<Statement, String> {
        self.expect_keyword("BACKUP")?;
        let path = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected path string, got {other:?}")),
        };
        Ok(Statement::VerifyBackup { path })
    }

    // ── v2.1: Comments ──

    fn parse_comment(&mut self) -> Result<Statement, String> {
        self.expect_keyword("ON")?;
        let target_type = self.expect_word()?; // BUNDLE, FIELD, CONSTRAINT
        let target = self.expect_word()?;
        // Handle dotted names like sensors.temp
        let target = if matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let field = self.expect_word()?;
            format!("{target}.{field}")
        } else {
            target
        };
        self.expect_keyword("IS")?;
        let comment = match self.advance() {
            Some(Token::Str(s)) => s,
            other => return Err(format!("Expected comment string, got {other:?}")),
        };
        Ok(Statement::CommentOn {
            target_type,
            target,
            comment,
        })
    }

    // ── v2.1: Recursive ──

    fn parse_iterate(&mut self) -> Result<Statement, String> {
        let bundle = self.expect_word()?;
        self.expect_keyword("START")?;
        self.expect_keyword("AT")?;
        let mut start_key = Vec::new();
        loop {
            let field = self.expect_word()?;
            self.expect(Token::Eq)?;
            let val = self.parse_literal()?;
            start_key.push((field, val));
            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }
            self.advance();
        }
        self.expect_keyword("STEP")?;
        self.expect_keyword("ALONG")?;
        let step_field = self.expect_word()?;
        let mut max_depth = None;
        // consume UNTIL VOID or UNTIL DEPTH n or MAX DEPTH n
        while !self.at_end() {
            if self.is_keyword("UNTIL") {
                self.advance();
                if self.is_keyword("VOID") {
                    self.advance();
                } else if self.is_keyword("DEPTH") {
                    self.advance();
                    max_depth = Some(self.parse_usize()?);
                }
            } else if self.is_keyword("MAX") {
                self.advance();
                self.expect_keyword("DEPTH")?;
                max_depth = Some(self.parse_usize()?);
            } else {
                break;
            }
        }
        Ok(Statement::Iterate {
            bundle,
            start_key,
            step_field,
            max_depth,
        })
    }

    // ── v2.1: Triggers ──

    fn parse_trigger(&mut self, keyword: &str) -> Result<Statement, String> {
        let event_prefix = keyword.to_ascii_uppercase();
        let event_action = self.expect_word()?; // SECTION, REDEFINE, RETRACT, CURVATURE, CONSISTENCY
        let bundle = self.expect_word()?;
        let mut condition = None;
        if self.is_keyword("WHERE") {
            self.advance();
            // Capture condition as raw string
            let mut parts = Vec::new();
            while !self.at_end()
                && !self.is_keyword("EXECUTE")
                && !self.is_keyword("CASCADE")
                && !self.is_keyword("CHECK")
            {
                match self.advance() {
                    Some(Token::Word(w)) => parts.push(w),
                    Some(Token::Number(n)) => parts.push(n.to_string()),
                    Some(Token::Str(s)) => parts.push(format!("'{s}'")),
                    Some(Token::Gt) => parts.push(">".into()),
                    Some(Token::Lt) => parts.push("<".into()),
                    Some(Token::Eq) => parts.push("=".into()),
                    Some(Token::Gte) => parts.push(">=".into()),
                    Some(Token::Lte) => parts.push("<=".into()),
                    Some(Token::Neq) => parts.push("!=".into()),
                    _ => break,
                }
            }
            condition = Some(parts.join(" "));
        }
        // Capture action
        let mut action_parts = Vec::new();
        while !self.at_end() {
            match self.advance() {
                Some(Token::Word(w)) => action_parts.push(w),
                Some(Token::Str(s)) => action_parts.push(format!("'{s}'")),
                Some(Token::Number(n)) => action_parts.push(n.to_string()),
                Some(Token::LParen) => action_parts.push("(".into()),
                Some(Token::RParen) => action_parts.push(")".into()),
                Some(Token::Comma) => action_parts.push(",".into()),
                Some(Token::Eq) => action_parts.push("=".into()),
                Some(Token::Colon) => action_parts.push(":".into()),
                Some(Token::Dot) => action_parts.push(".".into()),
                Some(Token::Star) => action_parts.push("*".into()),
                _ => break,
            }
        }
        let action = action_parts.join(" ");
        let event = format!("{event_prefix} {event_action}");
        Ok(Statement::CreateTrigger {
            event,
            bundle,
            condition,
            action,
        })
    }

    /// Parse: INVALIDATE CACHE [ON <bundle>]
    fn parse_invalidate_cache(&mut self) -> Result<Statement, String> {
        // Expect "CACHE"
        let word = self.expect_word()?;
        if word.to_ascii_uppercase() != "CACHE" {
            return Err(format!("Expected CACHE after INVALIDATE, got: {word}"));
        }
        // Optional: ON <bundle>
        let bundle = if self.is_keyword("ON") {
            self.advance();
            Some(self.expect_word()?)
        } else {
            None
        };
        Ok(Statement::InvalidateCache { bundle })
    }
}

/// Parse a GQL statement string into a Statement AST.
pub fn parse(input: &str) -> Result<Statement, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let stmt = parser.parse()?;
    if matches!(parser.peek(), Some(Token::Semicolon)) {
        parser.advance();
    }
    Ok(stmt)
}

/// Convert a Literal to a GIGI Value.
pub fn literal_to_value(lit: &Literal) -> crate::types::Value {
    match lit {
        Literal::Integer(n) => crate::types::Value::Integer(*n),
        Literal::Float(f) => crate::types::Value::Float(*f),
        Literal::Text(s) => crate::types::Value::Text(s.clone()),
        Literal::Bool(b) => crate::types::Value::Bool(*b),
        Literal::Null => crate::types::Value::Null,
    }
}

/// Convert a FieldSpec to a GIGI FieldDef.
pub fn spec_to_field_def(spec: &FieldSpec) -> crate::types::FieldDef {
    let mut fd = match spec.ftype.to_ascii_uppercase().as_str() {
        "INT" | "INTEGER" | "NUMERIC" => crate::types::FieldDef::numeric(&spec.name),
        "FLOAT" | "REAL" | "DOUBLE" => crate::types::FieldDef::numeric(&spec.name),
        "TEXT" | "VARCHAR" | "STRING" | "CATEGORICAL" => {
            crate::types::FieldDef::categorical(&spec.name)
        }
        "BOOL" | "BOOLEAN" => crate::types::FieldDef::categorical(&spec.name),
        "TIMESTAMP" => crate::types::FieldDef::numeric(&spec.name),
        _ => crate::types::FieldDef::categorical(&spec.name),
    };
    if let Some(r) = spec.range {
        fd = fd.with_range(r);
    }
    if let Some(ref d) = spec.default {
        fd = fd.with_default(literal_to_value(d));
    }
    fd
}

/// Convert an AdjacencySpec (parser AST) to an AdjacencyDef (types).
pub fn adj_spec_to_def(spec: &AdjacencySpec) -> crate::types::AdjacencyDef {
    let kind = match &spec.kind {
        AdjacencySpecKind::Equality { field } => crate::types::AdjacencyKind::Equality {
            field: field.clone(),
        },
        AdjacencySpecKind::Metric { field, radius } => crate::types::AdjacencyKind::Metric {
            field: field.clone(),
            radius: *radius,
        },
        AdjacencySpecKind::Threshold { field, threshold } => {
            crate::types::AdjacencyKind::Threshold {
                field: field.clone(),
                threshold: *threshold,
            }
        }
        AdjacencySpecKind::Transform {
            source_field,
            target_field,
            transform,
        } => {
            let tfn = match transform.to_ascii_lowercase().as_str() {
                "log10" => crate::types::TransformFn::Log10,
                _ => crate::types::TransformFn::Log10, // default fallback; scale/biofilm need args
            };
            crate::types::AdjacencyKind::Transform {
                source_field: source_field.clone(),
                target_field: target_field.clone(),
                transform: tfn,
            }
        }
    };
    crate::types::AdjacencyDef {
        name: spec.name.clone(),
        kind,
        weight: spec.weight,
    }
}

/// Convert a FilterCondition to a QueryCondition.
fn filter_to_query_condition(fc: &FilterCondition) -> crate::bundle::QueryCondition {
    use crate::bundle::QueryCondition as QC;
    match fc {
        FilterCondition::Eq(f, v) => QC::Eq(f.clone(), literal_to_value(v)),
        FilterCondition::Neq(f, v) => QC::Neq(f.clone(), literal_to_value(v)),
        FilterCondition::Gt(f, v) => QC::Gt(f.clone(), literal_to_value(v)),
        FilterCondition::Gte(f, v) => QC::Gte(f.clone(), literal_to_value(v)),
        FilterCondition::Lt(f, v) => QC::Lt(f.clone(), literal_to_value(v)),
        FilterCondition::Lte(f, v) => QC::Lte(f.clone(), literal_to_value(v)),
        FilterCondition::In(f, vs) => QC::In(f.clone(), vs.iter().map(literal_to_value).collect()),
        FilterCondition::NotIn(f, vs) => {
            QC::NotIn(f.clone(), vs.iter().map(literal_to_value).collect())
        }
        FilterCondition::Contains(f, s) => QC::Contains(f.clone(), s.clone()),
        FilterCondition::StartsWith(f, s) => QC::StartsWith(f.clone(), s.clone()),
        FilterCondition::EndsWith(f, s) => QC::EndsWith(f.clone(), s.clone()),
        FilterCondition::Matches(f, s) => QC::Regex(f.clone(), s.clone()),
        FilterCondition::Void(f) => QC::IsNull(f.clone()),
        FilterCondition::Defined(f) => QC::IsNotNull(f.clone()),
        FilterCondition::Between(f, lo, _hi) => {
            // Between is desugared into Gte + Lte by filter_to_query_conditions()
            QC::Gte(f.clone(), literal_to_value(lo))
        }
    }
}

/// Convert FilterCondition::Between into two QueryConditions.
fn filter_to_query_conditions(fc: &FilterCondition) -> Vec<crate::bundle::QueryCondition> {
    use crate::bundle::QueryCondition as QC;
    match fc {
        FilterCondition::Between(f, lo, hi) => vec![
            QC::Gte(f.clone(), literal_to_value(lo)),
            QC::Lte(f.clone(), literal_to_value(hi)),
        ],
        other => vec![filter_to_query_condition(other)],
    }
}

// ── Execution ──

/// Execution result.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecResult {
    Ok,
    Rows(Vec<crate::types::Record>),
    Scalar(f64),
    Bool(bool),
    Count(usize),
    Stats(GqlStats),
    Bundles(Vec<GqlBundleInfo>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GqlStats {
    pub curvature: f64,
    pub confidence: f64,
    pub record_count: usize,
    pub storage_mode: String,
    pub base_fields: usize,
    pub fiber_fields: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GqlBundleInfo {
    pub name: String,
    pub records: usize,
    pub fields: usize,
}

/// Execute a parsed statement against an Engine.
pub fn execute(engine: &mut crate::engine::Engine, stmt: &Statement) -> Result<ExecResult, String> {
    match stmt {
        // ── Schema ──
        Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
        } => {
            let mut schema = crate::types::BundleSchema::new(name);
            for f in base_fields {
                schema = schema.base(spec_to_field_def(f));
            }
            for f in fiber_fields {
                schema = schema.fiber(spec_to_field_def(f));
            }
            for idx in indexed {
                schema = schema.index(idx);
            }
            for adj in adjacencies {
                schema = schema.adjacency(adj_spec_to_def(adj));
            }
            if *encrypted {
                let seed = crate::crypto::GaugeKey::random_seed();
                let gk = crate::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
                schema.gauge_key = Some(gk);
            }
            engine.create_bundle(schema).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::Collapse { bundle } => {
            engine.drop_bundle(bundle).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::ShowBundles => {
            let infos: Vec<GqlBundleInfo> = engine
                .bundle_names()
                .iter()
                .map(|name| {
                    let store = engine.bundle(name).unwrap();
                    GqlBundleInfo {
                        name: name.to_string(),
                        records: store.len(),
                        fields: store.schema.base_fields.len() + store.schema.fiber_fields.len(),
                    }
                })
                .collect();
            Ok(ExecResult::Bundles(infos))
        }

        Statement::Describe { bundle, verbose: _ } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = crate::curvature::scalar_curvature(store);
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: crate::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema.base_fields.len(),
                fiber_fields: store.schema.fiber_fields.len(),
            }))
        }

        // ── Write ──
        Statement::Insert {
            bundle,
            columns,
            values,
        } => {
            if !columns.is_empty() && columns.len() != values.len() {
                return Err("Column count doesn't match value count".into());
            }
            let mut record = HashMap::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                record.insert(col.clone(), literal_to_value(val));
            }
            engine.insert(bundle, &record).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::BatchInsert {
            bundle,
            columns,
            rows,
        } => {
            let records: Vec<crate::types::Record> = rows
                .iter()
                .map(|row| {
                    if columns.is_empty() {
                        // Positional — use schema field order
                        row.iter()
                            .enumerate()
                            .map(|(i, v)| (format!("_{i}"), literal_to_value(v)))
                            .collect()
                    } else {
                        columns
                            .iter()
                            .zip(row.iter())
                            .map(|(c, v)| (c.clone(), literal_to_value(v)))
                            .collect()
                    }
                })
                .collect();
            engine
                .batch_insert(bundle, &records)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::SectionUpsert {
            bundle,
            columns,
            values,
        } => {
            let mut record = HashMap::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                record.insert(col.clone(), literal_to_value(val));
            }
            let store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            store.upsert(&record);
            Ok(ExecResult::Ok)
        }

        Statement::Redefine { bundle, key, sets } => {
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let patches: crate::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let updated = engine
                .update(bundle, &key_rec, &patches)
                .map_err(|e| format!("{e}"))?;
            if updated {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }

        Statement::BulkRedefine {
            bundle,
            conditions,
            sets,
        } => {
            let qcs: Vec<crate::bundle::QueryCondition> = conditions
                .iter()
                .flat_map(filter_to_query_conditions)
                .collect();
            let patches: crate::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let matched = store.bulk_update(&qcs, &patches);
            Ok(ExecResult::Count(matched))
        }

        Statement::Retract { bundle, key } => {
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let deleted = engine
                .delete(bundle, &key_rec)
                .map_err(|e| format!("{e}"))?;
            if deleted {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }

        Statement::BulkRetract { bundle, conditions } => {
            let qcs: Vec<crate::bundle::QueryCondition> = conditions
                .iter()
                .flat_map(filter_to_query_conditions)
                .collect();
            let store = engine
                .bundle_mut(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let deleted = store.bulk_delete(&qcs);
            Ok(ExecResult::Count(deleted))
        }

        // ── Point Query ──
        Statement::PointQuery {
            bundle,
            key,
            project,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            match store.point_query(&key_rec) {
                Some(mut rec) => {
                    if let Some(fields) = project {
                        rec.retain(|k, _| fields.contains(k));
                    }
                    Ok(ExecResult::Rows(vec![rec]))
                }
                None => Ok(ExecResult::Rows(vec![])),
            }
        }

        Statement::ExistsSection { bundle, key } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let key_rec: crate::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            Ok(ExecResult::Bool(store.point_query(&key_rec).is_some()))
        }

        // ── Cover/Range Query ──
        Statement::Cover {
            bundle,
            on_conditions,
            where_conditions,
            or_groups,
            distinct_field,
            project,
            rank_by,
            first,
            skip,
            all: _,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            // Handle DISTINCT
            if let Some(field) = distinct_field {
                let vals = store.distinct(field);
                let rows: Vec<crate::types::Record> = vals
                    .into_iter()
                    .map(|v| {
                        let mut r = HashMap::new();
                        r.insert(field.clone(), v);
                        r
                    })
                    .collect();
                return Ok(ExecResult::Rows(rows));
            }

            // Build conditions
            let mut conditions: Vec<crate::bundle::QueryCondition> = Vec::new();
            for fc in on_conditions.iter().chain(where_conditions.iter()) {
                conditions.extend(filter_to_query_conditions(fc));
            }

            let or_qcs: Vec<Vec<crate::bundle::QueryCondition>> = or_groups
                .iter()
                .map(|group| group.iter().flat_map(filter_to_query_conditions).collect())
                .collect();

            let or_ref = if or_qcs.is_empty() {
                None
            } else {
                Some(or_qcs.as_slice())
            };

            // Use projected query if PROJECT specified
            let results = if let Some(fields) = project {
                let sort_refs: Vec<(&str, bool)> = rank_by
                    .as_ref()
                    .map(|specs| specs.iter().map(|s| (s.field.as_str(), s.desc)).collect())
                    .unwrap_or_default();
                let sort_opt = if sort_refs.is_empty() {
                    None
                } else {
                    Some(sort_refs.as_slice())
                };
                let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                let (rows, _total) = store.filtered_query_projected_ex(
                    &conditions,
                    or_ref,
                    sort_opt,
                    *first,
                    *skip,
                    Some(&field_refs),
                );
                rows
            } else {
                // Use simple filtered_query_ex with single sort field
                let (sort_by, sort_desc) = rank_by
                    .as_ref()
                    .and_then(|specs| specs.first())
                    .map(|s| (Some(s.field.as_str()), s.desc))
                    .unwrap_or((None, false));
                store.filtered_query_ex(&conditions, or_ref, sort_by, sort_desc, *first, *skip)
            };

            Ok(ExecResult::Rows(results))
        }

        // ── Aggregation ──
        Statement::Integrate {
            bundle,
            over,
            measures,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            if let Some(gb_field) = over {
                let agg_field = measures.first().map(|m| m.field.as_str()).unwrap_or("*");

                let groups = crate::aggregation::group_by(store, gb_field, agg_field);
                let mut rows = Vec::new();
                for (key, agg_result) in &groups {
                    let mut row = HashMap::new();
                    row.insert(gb_field.clone(), key.clone());
                    for m in measures {
                        let val = match m.func {
                            AggFunc::Count => agg_result.count as f64,
                            AggFunc::Sum => agg_result.sum,
                            AggFunc::Avg => agg_result.avg(),
                            AggFunc::Min => agg_result.min,
                            AggFunc::Max => agg_result.max,
                        };
                        let field_name = m
                            .alias
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field));
                        row.insert(field_name, crate::types::Value::Float(val));
                    }
                    rows.push(row);
                }
                Ok(ExecResult::Rows(rows))
            } else {
                // Global aggregation — no OVER
                let all: Vec<crate::types::Record> = store.records().collect();
                let mut row = HashMap::new();
                for m in measures {
                    let vals: Vec<f64> = all
                        .iter()
                        .filter_map(|r| r.get(&m.field))
                        .filter_map(|v| match v {
                            crate::types::Value::Integer(n) => Some(*n as f64),
                            crate::types::Value::Float(f) => Some(*f),
                            _ => None,
                        })
                        .collect();
                    let val = match m.func {
                        AggFunc::Count => vals.len() as f64,
                        AggFunc::Sum => vals.iter().sum(),
                        AggFunc::Avg => {
                            if vals.is_empty() {
                                0.0
                            } else {
                                vals.iter().sum::<f64>() / vals.len() as f64
                            }
                        }
                        AggFunc::Min => vals.iter().cloned().fold(f64::INFINITY, f64::min),
                        AggFunc::Max => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    };
                    let field_name = m
                        .alias
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field));
                    row.insert(field_name, crate::types::Value::Float(val));
                }
                Ok(ExecResult::Rows(vec![row]))
            }
        }

        // ── Joins ──
        Statement::Pullback {
            left,
            along,
            right,
            right_field,
            preserve_left: _,
        } => {
            let left_store = engine
                .bundle(left)
                .ok_or_else(|| format!("No bundle: {left}"))?;
            let right_store = engine
                .bundle(right)
                .ok_or_else(|| format!("No bundle: {right}"))?;
            let rf = right_field.as_deref().unwrap_or(along.as_str());
            let joined = crate::join::pullback_join(left_store, right_store, along, rf);
            let rows: Vec<_> = joined
                .into_iter()
                .map(|(left_rec, right_rec)| {
                    let mut merged = left_rec;
                    if let Some(r) = right_rec {
                        merged.extend(r);
                    }
                    merged
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }

        // ── SQL compat: SELECT ──
        Statement::Select {
            bundle,
            columns,
            condition,
            group_by,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;

            if let Some(gb_field) = group_by {
                let agg_col = columns.iter().find_map(|c| match c {
                    SelectCol::Agg(func, field) => Some((func, field)),
                    _ => None,
                });
                if let Some((func, field)) = agg_col {
                    let groups = crate::aggregation::group_by(store, gb_field, field);
                    let mut rows = Vec::new();
                    for (key, agg_result) in &groups {
                        let mut row = HashMap::new();
                        row.insert(gb_field.clone(), key.clone());
                        let val = match func {
                            AggFunc::Count => agg_result.count as f64,
                            AggFunc::Sum => agg_result.sum,
                            AggFunc::Avg => agg_result.avg(),
                            AggFunc::Min => agg_result.min,
                            AggFunc::Max => agg_result.max,
                        };
                        row.insert(field.clone(), crate::types::Value::Float(val));
                        rows.push(row);
                    }
                    return Ok(ExecResult::Rows(rows));
                }
            }

            match condition {
                Some(Condition::Eq(field, val)) => {
                    let value = literal_to_value(val);
                    let is_base = store.schema.base_fields.iter().any(|f| f.name == *field);
                    if is_base {
                        let mut key = HashMap::new();
                        key.insert(field.clone(), value);
                        match store.point_query(&key) {
                            Some(rec) => Ok(ExecResult::Rows(vec![filter_columns(rec, columns)])),
                            None => Ok(ExecResult::Rows(vec![])),
                        }
                    } else {
                        let results = store.range_query(field, &[value]);
                        let rows: Vec<_> = results
                            .into_iter()
                            .map(|r| filter_columns(r, columns))
                            .collect();
                        Ok(ExecResult::Rows(rows))
                    }
                }
                Some(Condition::Between(field, lo, hi)) => {
                    let lo_val = literal_to_value(lo);
                    let hi_val = literal_to_value(hi);
                    let matching: Vec<crate::types::Value> = store
                        .indexed_values(field)
                        .into_iter()
                        .filter(|v| *v >= lo_val && *v <= hi_val)
                        .collect();
                    let results = store.range_query(field, &matching);
                    let rows: Vec<_> = results
                        .into_iter()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
                Some(Condition::In(field, vals)) => {
                    let values: Vec<_> = vals.iter().map(literal_to_value).collect();
                    let results = store.range_query(field, &values);
                    let rows: Vec<_> = results
                        .into_iter()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
                None => {
                    let rows: Vec<_> = store
                        .records()
                        .map(|r| filter_columns(r, columns))
                        .collect();
                    Ok(ExecResult::Rows(rows))
                }
            }
        }

        // ── SQL compat: JOIN ──
        Statement::Join {
            left,
            right,
            on_field,
            columns,
        } => {
            let left_store = engine
                .bundle(left)
                .ok_or_else(|| format!("No bundle: {left}"))?;
            let right_store = engine
                .bundle(right)
                .ok_or_else(|| format!("No bundle: {right}"))?;
            let joined = crate::join::pullback_join(left_store, right_store, on_field, on_field);
            let rows: Vec<_> = joined
                .into_iter()
                .map(|(left_rec, right_rec)| {
                    let mut merged = left_rec;
                    if let Some(r) = right_rec {
                        merged.extend(r);
                    }
                    filter_columns(merged, columns)
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }

        // ── Analytics ──
        Statement::Curvature { bundle, .. } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = crate::curvature::scalar_curvature(store);
            Ok(ExecResult::Scalar(k))
        }

        Statement::Spectral { bundle, .. } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let lambda1 = crate::spectral::spectral_gap(store);
            Ok(ExecResult::Scalar(lambda1))
        }

        Statement::Consistency { bundle, repair: _ } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let contradictions = crate::sheaf::consistency_check(store);
            Ok(ExecResult::Rows(contradictions))
        }

        Statement::Complete {
            bundle,
            where_conditions,
            method: _,
            min_confidence,
            with_provenance,
            with_constraint_graph,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let min_conf = min_confidence.unwrap_or(0.30);
            let results = crate::sheaf::complete(
                store,
                where_conditions,
                min_conf,
                *with_provenance,
                *with_constraint_graph,
            );
            Ok(ExecResult::Rows(results))
        }

        Statement::Propagate {
            bundle,
            assumptions,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let assumption_record = assumptions
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect::<crate::types::Record>();
            let results = crate::sheaf::propagate(store, &assumption_record);
            Ok(ExecResult::Rows(results))
        }

        Statement::SuggestAdjacency {
            bundle,
            fields,
            sample_size,
            candidates,
        } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let results = crate::sheaf::suggest_adjacency(store, fields, *sample_size, *candidates);
            Ok(ExecResult::Rows(results))
        }

        Statement::Health { bundle } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            let k = crate::curvature::scalar_curvature(store);
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: crate::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema.base_fields.len(),
                fiber_fields: store.schema.fiber_fields.len(),
            }))
        }

        Statement::Explain { inner: _ } => {
            // Query plan introspection — placeholder
            Ok(ExecResult::Ok)
        }

        Statement::AtlasBegin | Statement::AtlasCommit | Statement::AtlasRollback => {
            // Transaction control is handled at the transport layer
            Ok(ExecResult::Ok)
        }

        // ── v2.1: Access Control (stubs) ──
        Statement::WeaveRole { .. }
        | Statement::UnweaveRole { .. }
        | Statement::ShowRoles
        | Statement::Grant { .. }
        | Statement::Revoke { .. }
        | Statement::CreatePolicy { .. }
        | Statement::DropPolicy { .. }
        | Statement::ShowPolicies { .. }
        | Statement::AuditOn { .. }
        | Statement::AuditOff { .. }
        | Statement::AuditShow { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Constraints (stubs) ──
        Statement::GaugeConstrain { .. }
        | Statement::GaugeUnconstrain { .. }
        | Statement::ShowConstraints { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Maintenance ──
        Statement::Compact { bundle, .. }
        | Statement::Analyze { bundle, .. }
        | Statement::Vacuum { bundle, .. }
        | Statement::RebuildIndex { bundle, .. }
        | Statement::CheckIntegrity { bundle }
        | Statement::Repair { bundle } => {
            let _store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            Ok(ExecResult::Ok)
        }

        Statement::StorageInfo { bundle } => {
            let store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            Ok(ExecResult::Stats(GqlStats {
                curvature: crate::curvature::scalar_curvature(store),
                confidence: 0.0,
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema.base_fields.len(),
                fiber_fields: store.schema.fiber_fields.len(),
            }))
        }

        // ── v2.1: Session (stubs) ──
        Statement::Set { .. }
        | Statement::Reset { .. }
        | Statement::ShowSettings
        | Statement::ShowSession
        | Statement::ShowCurrentRole => Ok(ExecResult::Ok),

        // ── v2.1: Data Movement (stubs) ──
        Statement::Ingest { .. }
        | Statement::Transplant { .. }
        | Statement::GenerateBase { .. }
        | Statement::Fill { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Prepared Statements (stubs) ──
        Statement::Prepare { .. }
        | Statement::Execute { .. }
        | Statement::Deallocate { .. }
        | Statement::ShowPrepared => Ok(ExecResult::Ok),

        // ── v2.1: Backup / Restore (stubs) ──
        Statement::Backup { .. }
        | Statement::Restore { .. }
        | Statement::VerifyBackup { .. }
        | Statement::ShowBackups => Ok(ExecResult::Ok),

        // ── v2.1: Information Schema ──
        Statement::ShowFields { bundle }
        | Statement::ShowIndexes { bundle }
        | Statement::ShowMorphisms { bundle }
        | Statement::ShowTriggers { bundle }
        | Statement::ShowStatistics { bundle }
        | Statement::ShowGeometry { bundle }
        | Statement::ShowComments { bundle } => {
            let _store = engine
                .bundle(bundle)
                .ok_or_else(|| format!("No bundle: {bundle}"))?;
            Ok(ExecResult::Ok)
        }

        // ── v2.1: Comments (stub) ──
        Statement::CommentOn { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Recursive (stub) ──
        Statement::Iterate { .. } => Ok(ExecResult::Ok),

        // ── v2.1: Triggers (stubs) ──
        Statement::CreateTrigger { .. } | Statement::DropTrigger { .. } => Ok(ExecResult::Ok),

        // ── Feature #6: Query Cache ──
        Statement::InvalidateCache { bundle } => {
            if let Some(b) = bundle {
                engine.query_cache_mut().invalidate_bundle(b);
            } else {
                engine.query_cache_mut().invalidate_all();
            }
            Ok(ExecResult::Ok)
        }
    }
}

impl MeasureSpec {
    pub fn func_name(&self) -> &str {
        match self.func {
            AggFunc::Count => "count",
            AggFunc::Sum => "sum",
            AggFunc::Avg => "avg",
            AggFunc::Min => "min",
            AggFunc::Max => "max",
        }
    }
}

fn filter_columns(record: crate::types::Record, columns: &[SelectCol]) -> crate::types::Record {
    if columns.iter().any(|c| matches!(c, SelectCol::Star)) {
        return record;
    }
    let mut filtered = HashMap::new();
    for col in columns {
        if let SelectCol::Name(name) = col {
            if let Some(v) = record.get(name) {
                filtered.insert(name.clone(), v.clone());
            }
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SQL compat tests (existing) ──

    #[test]
    fn parse_create_bundle() {
        let stmt = parse("CREATE BUNDLE employees (id INT BASE, name TEXT FIBER, salary FLOAT RANGE(100000) FIBER INDEX)").unwrap();
        match stmt {
            Statement::CreateBundle {
                name,
                base_fields,
                fiber_fields,
                indexed,
                ..
            } => {
                assert_eq!(name, "employees");
                assert_eq!(base_fields.len(), 1);
                assert_eq!(base_fields[0].name, "id");
                assert_eq!(fiber_fields.len(), 2);
                assert_eq!(fiber_fields[1].name, "salary");
                assert_eq!(fiber_fields[1].range, Some(100000.0));
                assert_eq!(indexed, vec!["salary"]);
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    #[test]
    fn parse_insert() {
        let stmt =
            parse("INSERT INTO employees (id, name, salary) VALUES (1, 'Alice', 75000.0)").unwrap();
        match stmt {
            Statement::Insert {
                bundle,
                columns,
                values,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(columns, vec!["id", "name", "salary"]);
                assert_eq!(values[0], Literal::Integer(1));
                assert_eq!(values[1], Literal::Text("Alice".into()));
                assert_eq!(values[2], Literal::Integer(75000));
            }
            _ => panic!("Expected Insert"),
        }
    }

    #[test]
    fn parse_select_point_query() {
        let stmt = parse("SELECT * FROM employees WHERE id = 1").unwrap();
        match stmt {
            Statement::Select {
                bundle,
                columns,
                condition,
                group_by,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(columns, vec![SelectCol::Star]);
                assert_eq!(
                    condition,
                    Some(Condition::Eq("id".into(), Literal::Integer(1)))
                );
                assert!(group_by.is_none());
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_select_range() {
        let stmt =
            parse("SELECT name, salary FROM employees WHERE salary BETWEEN 50000 AND 100000")
                .unwrap();
        match stmt {
            Statement::Select { condition, .. } => {
                assert_eq!(
                    condition,
                    Some(Condition::Between(
                        "salary".into(),
                        Literal::Integer(50000),
                        Literal::Integer(100000)
                    ))
                );
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_select_group_by() {
        let stmt = parse("SELECT dept, AVG(salary) FROM employees GROUP BY dept").unwrap();
        match stmt {
            Statement::Select {
                columns, group_by, ..
            } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0], SelectCol::Name("dept".into()));
                assert_eq!(columns[1], SelectCol::Agg(AggFunc::Avg, "salary".into()));
                assert_eq!(group_by, Some("dept".into()));
            }
            _ => panic!("Expected Select"),
        }
    }

    #[test]
    fn parse_join() {
        let stmt = parse("SELECT * FROM orders JOIN customers ON customer_id").unwrap();
        match stmt {
            Statement::Join {
                left,
                right,
                on_field,
                ..
            } => {
                assert_eq!(left, "orders");
                assert_eq!(right, "customers");
                assert_eq!(on_field, "customer_id");
            }
            _ => panic!("Expected Join"),
        }
    }

    #[test]
    fn parse_curvature_spectral() {
        assert!(matches!(
            parse("CURVATURE employees").unwrap(),
            Statement::Curvature { .. }
        ));
        assert!(matches!(
            parse("SPECTRAL employees").unwrap(),
            Statement::Spectral { .. }
        ));
    }

    #[test]
    fn execute_full_workflow() {
        let dir = std::env::temp_dir().join("gigi_parser_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = crate::engine::Engine::open(&dir).unwrap();

        // Create bundle
        let stmt = parse("CREATE BUNDLE emp (id INT BASE, name TEXT FIBER, salary FLOAT RANGE(100000) FIBER INDEX)").unwrap();
        execute(&mut engine, &stmt).unwrap();

        // Insert
        for i in 0..5 {
            let sql = format!(
                "INSERT INTO emp (id, name, salary) VALUES ({i}, 'Person{i}', {})",
                50000.0 + i as f64 * 10000.0
            );
            let stmt = parse(&sql).unwrap();
            execute(&mut engine, &stmt).unwrap();
        }

        // Point query
        let stmt = parse("SELECT * FROM emp WHERE id = 0").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 1),
            _ => panic!("Expected rows"),
        }

        // Full scan
        let stmt = parse("SELECT * FROM emp").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // Curvature
        let stmt = parse("CURVATURE emp").unwrap();
        let result = execute(&mut engine, &stmt).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GQL Native tests ──

    #[test]
    fn gql_bundle_keyword_style() {
        let stmt = parse("BUNDLE sensors BASE (id NUMERIC) FIBER (city CATEGORICAL INDEX, temp NUMERIC RANGE 80)").unwrap();
        match stmt {
            Statement::CreateBundle {
                name,
                base_fields,
                fiber_fields,
                indexed,
                ..
            } => {
                assert_eq!(name, "sensors");
                assert_eq!(base_fields.len(), 1);
                assert_eq!(base_fields[0].name, "id");
                assert_eq!(fiber_fields.len(), 2);
                assert_eq!(fiber_fields[0].name, "city");
                assert_eq!(fiber_fields[1].range, Some(80.0));
                assert_eq!(indexed, vec!["city"]);
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    #[test]
    fn gql_section_insert() {
        let stmt = parse("SECTION sensors (id: 42, city: 'Moscow', temp: -31.9)").unwrap();
        match stmt {
            Statement::Insert {
                bundle,
                columns,
                values,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(columns, vec!["id", "city", "temp"]);
                assert_eq!(values[0], Literal::Integer(42));
                assert_eq!(values[1], Literal::Text("Moscow".into()));
            }
            _ => panic!("Expected Insert"),
        }
    }

    #[test]
    fn gql_section_upsert() {
        let stmt = parse("SECTION sensors (id: 42, city: 'Moscow', temp: -28.5) UPSERT").unwrap();
        assert!(matches!(stmt, Statement::SectionUpsert { .. }));
    }

    #[test]
    fn gql_section_point_query() {
        let stmt = parse("SECTION sensors AT id=42").unwrap();
        match stmt {
            Statement::PointQuery {
                bundle,
                key,
                project,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key, vec![("id".into(), Literal::Integer(42))]);
                assert!(project.is_none());
            }
            _ => panic!("Expected PointQuery"),
        }
    }

    #[test]
    fn gql_section_projected() {
        let stmt = parse("SECTION sensors AT id=42 PROJECT (city, temp)").unwrap();
        match stmt {
            Statement::PointQuery { project, .. } => {
                assert_eq!(project, Some(vec!["city".into(), "temp".into()]));
            }
            _ => panic!("Expected PointQuery"),
        }
    }

    #[test]
    fn gql_exists_section() {
        let stmt = parse("EXISTS SECTION sensors AT id=42").unwrap();
        assert!(matches!(stmt, Statement::ExistsSection { .. }));
    }

    #[test]
    fn gql_redefine_point() {
        let stmt = parse("REDEFINE sensors AT id=42 SET (temp: -28.5)").unwrap();
        match stmt {
            Statement::Redefine { bundle, key, sets } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key[0], ("id".into(), Literal::Integer(42)));
                assert_eq!(sets[0].0, "temp");
            }
            _ => panic!("Expected Redefine"),
        }
    }

    #[test]
    fn gql_retract() {
        let stmt = parse("RETRACT sensors AT id=42").unwrap();
        match stmt {
            Statement::Retract { bundle, key } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(key[0], ("id".into(), Literal::Integer(42)));
            }
            _ => panic!("Expected Retract"),
        }
    }

    #[test]
    fn gql_cover_on() {
        let stmt = parse("COVER sensors ON city = 'Moscow'").unwrap();
        match stmt {
            Statement::Cover {
                bundle,
                on_conditions,
                ..
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(on_conditions.len(), 1);
                assert_eq!(
                    on_conditions[0],
                    FilterCondition::Eq("city".into(), Literal::Text("Moscow".into()))
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_where_lt() {
        let stmt = parse("COVER sensors WHERE temp < -25").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(where_conditions.len(), 1);
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Lt("temp".into(), Literal::Integer(-25))
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_on_where_combined() {
        let stmt = parse("COVER sensors ON city = 'Moscow' WHERE temp < -25").unwrap();
        match stmt {
            Statement::Cover {
                on_conditions,
                where_conditions,
                ..
            } => {
                assert_eq!(on_conditions.len(), 1);
                assert_eq!(where_conditions.len(), 1);
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_distinct() {
        let stmt = parse("COVER sensors DISTINCT city").unwrap();
        match stmt {
            Statement::Cover { distinct_field, .. } => {
                assert_eq!(distinct_field, Some("city".into()));
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_rank_first_skip() {
        let stmt = parse("COVER sensors RANK BY temp DESC SKIP 10 FIRST 5").unwrap();
        match stmt {
            Statement::Cover {
                rank_by,
                skip,
                first,
                ..
            } => {
                let sort = rank_by.unwrap();
                assert_eq!(sort[0].field, "temp");
                assert!(sort[0].desc);
                assert_eq!(skip, Some(10));
                assert_eq!(first, Some(5));
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_all() {
        let stmt = parse("COVER sensors ALL").unwrap();
        match stmt {
            Statement::Cover { all, .. } => assert!(all),
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_in() {
        let stmt = parse("COVER sensors ON region IN ('EU', 'NA')").unwrap();
        match stmt {
            Statement::Cover { on_conditions, .. } => {
                assert_eq!(
                    on_conditions[0],
                    FilterCondition::In(
                        "region".into(),
                        vec![Literal::Text("EU".into()), Literal::Text("NA".into())]
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_void_defined() {
        let stmt = parse("COVER sensors WHERE pressure VOID").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Void("pressure".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
        let stmt = parse("COVER sensors WHERE pressure DEFINED").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Defined("pressure".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_matches() {
        let stmt = parse("COVER sensors WHERE city MATCHES 'Mos*'").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Matches("city".into(), "Mos*".into())
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_project() {
        let stmt = parse("COVER sensors ON city = 'Moscow' PROJECT (city, temp, wind)").unwrap();
        match stmt {
            Statement::Cover { project, .. } => {
                assert_eq!(
                    project,
                    Some(vec!["city".into(), "temp".into(), "wind".into()])
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_integrate_over_measure() {
        let stmt = parse("INTEGRATE sensors OVER city MEASURE avg(temp), count(*)").unwrap();
        match stmt {
            Statement::Integrate {
                bundle,
                over,
                measures,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(over, Some("city".into()));
                assert_eq!(measures.len(), 2);
                assert_eq!(measures[0].field, "temp");
                assert!(matches!(measures[0].func, AggFunc::Avg));
                assert_eq!(measures[1].field, "*");
                assert!(matches!(measures[1].func, AggFunc::Count));
            }
            _ => panic!("Expected Integrate"),
        }
    }

    #[test]
    fn gql_integrate_global() {
        let stmt = parse("INTEGRATE sensors MEASURE avg(temp), max(wind)").unwrap();
        match stmt {
            Statement::Integrate { over, measures, .. } => {
                assert!(over.is_none());
                assert_eq!(measures.len(), 2);
            }
            _ => panic!("Expected Integrate"),
        }
    }

    #[test]
    fn gql_pullback() {
        let stmt = parse("PULLBACK readings ALONG sensor_id ONTO sensors").unwrap();
        match stmt {
            Statement::Pullback {
                left, along, right, ..
            } => {
                assert_eq!(left, "readings");
                assert_eq!(along, "sensor_id");
                assert_eq!(right, "sensors");
            }
            _ => panic!("Expected Pullback"),
        }
    }

    #[test]
    fn gql_pullback_preserve_left() {
        let stmt = parse("PULLBACK readings ALONG sensor_id ONTO sensors PRESERVE LEFT").unwrap();
        match stmt {
            Statement::Pullback { preserve_left, .. } => assert!(preserve_left),
            _ => panic!("Expected Pullback"),
        }
    }

    #[test]
    fn gql_curvature_fields_by() {
        let stmt = parse("CURVATURE sensors ON temp, wind BY city").unwrap();
        match stmt {
            Statement::Curvature {
                bundle,
                fields,
                by_field,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(fields, vec!["temp", "wind"]);
                assert_eq!(by_field, Some("city".into()));
            }
            _ => panic!("Expected Curvature"),
        }
    }

    #[test]
    fn gql_spectral_full() {
        let stmt = parse("SPECTRAL sensors FULL").unwrap();
        match stmt {
            Statement::Spectral { bundle, full } => {
                assert_eq!(bundle, "sensors");
                assert!(full);
            }
            _ => panic!("Expected Spectral"),
        }
    }

    #[test]
    fn gql_consistency_repair() {
        let stmt = parse("CONSISTENCY sensors REPAIR").unwrap();
        match stmt {
            Statement::Consistency { bundle, repair } => {
                assert_eq!(bundle, "sensors");
                assert!(repair);
            }
            _ => panic!("Expected Consistency"),
        }
    }

    #[test]
    fn gql_suggest_adjacency_basic() {
        let stmt = parse("SUGGEST_ADJACENCY ON chembl_activities MINIMIZING h1").unwrap();
        match stmt {
            Statement::SuggestAdjacency {
                bundle,
                fields,
                sample_size,
                candidates,
            } => {
                assert_eq!(bundle, "chembl_activities");
                assert!(fields.is_empty());
                assert_eq!(sample_size, 10_000);
                assert_eq!(candidates, 5);
            }
            _ => panic!("Expected SuggestAdjacency"),
        }
    }

    #[test]
    fn gql_suggest_adjacency_full() {
        let stmt = parse(
            "SUGGEST_ADJACENCY ON mydata FIELDS pchembl_value, assay_type SAMPLE_SIZE 5000 CANDIDATES 10 MINIMIZING h1",
        )
        .unwrap();
        match stmt {
            Statement::SuggestAdjacency {
                bundle,
                fields,
                sample_size,
                candidates,
            } => {
                assert_eq!(bundle, "mydata");
                assert_eq!(fields, vec!["pchembl_value", "assay_type"]);
                assert_eq!(sample_size, 5000);
                assert_eq!(candidates, 10);
            }
            _ => panic!("Expected SuggestAdjacency"),
        }
    }

    #[test]
    fn gql_show_bundles() {
        assert!(matches!(
            parse("SHOW BUNDLES").unwrap(),
            Statement::ShowBundles
        ));
    }

    #[test]
    fn gql_describe() {
        let stmt = parse("DESCRIBE sensors VERBOSE").unwrap();
        match stmt {
            Statement::Describe { bundle, verbose } => {
                assert_eq!(bundle, "sensors");
                assert!(verbose);
            }
            _ => panic!("Expected Describe"),
        }
    }

    #[test]
    fn gql_collapse() {
        let stmt = parse("COLLAPSE sensors").unwrap();
        match stmt {
            Statement::Collapse { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected Collapse"),
        }
    }

    #[test]
    fn gql_health() {
        let stmt = parse("HEALTH sensors").unwrap();
        assert!(matches!(stmt, Statement::Health { .. }));
    }

    #[test]
    fn gql_explain() {
        let stmt = parse("EXPLAIN COVER sensors ON city = 'Moscow'").unwrap();
        match stmt {
            Statement::Explain { inner } => {
                assert!(matches!(*inner, Statement::Cover { .. }));
            }
            _ => panic!("Expected Explain"),
        }
    }

    #[test]
    fn gql_atlas_begin_commit() {
        assert!(matches!(
            parse("ATLAS BEGIN").unwrap(),
            Statement::AtlasBegin
        ));
        assert!(matches!(
            parse("ATLAS COMMIT").unwrap(),
            Statement::AtlasCommit
        ));
        assert!(matches!(
            parse("ATLAS ROLLBACK").unwrap(),
            Statement::AtlasRollback
        ));
    }

    #[test]
    fn gql_cover_between() {
        let stmt = parse("COVER sensors WHERE temp BETWEEN -30 AND 0").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::Between(
                        "temp".into(),
                        Literal::Integer(-30),
                        Literal::Integer(0)
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_cover_not_in() {
        let stmt = parse("COVER sensors WHERE region NOT IN ('TEST', 'DEV')").unwrap();
        match stmt {
            Statement::Cover {
                where_conditions, ..
            } => {
                assert_eq!(
                    where_conditions[0],
                    FilterCondition::NotIn(
                        "region".into(),
                        vec![Literal::Text("TEST".into()), Literal::Text("DEV".into())]
                    )
                );
            }
            _ => panic!("Expected Cover"),
        }
    }

    #[test]
    fn gql_line_comment_ignored() {
        let stmt = parse("-- this is a comment\nSHOW BUNDLES").unwrap();
        assert!(matches!(stmt, Statement::ShowBundles));
    }

    #[test]
    fn gql_execute_full_workflow() {
        let dir = std::env::temp_dir().join("gigi_gql_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = crate::engine::Engine::open(&dir).unwrap();

        // BUNDLE (keyword style)
        let stmt = parse("BUNDLE emp BASE (id NUMERIC) FIBER (name CATEGORICAL, salary NUMERIC RANGE 100000 INDEX, dept CATEGORICAL INDEX)").unwrap();
        execute(&mut engine, &stmt).unwrap();

        // SECTION (insert)
        for i in 0..5 {
            let gql = format!(
                "SECTION emp (id: {i}, name: 'Person{i}', salary: {}, dept: 'Eng')",
                50000 + i * 10000
            );
            execute(&mut engine, &parse(&gql).unwrap()).unwrap();
        }

        // SECTION AT (point query)
        let result = execute(&mut engine, &parse("SECTION emp AT id=0").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(
                    rows[0].get("name"),
                    Some(&crate::types::Value::Text("Person0".into()))
                );
            }
            _ => panic!("Expected rows"),
        }

        // SECTION AT ... PROJECT
        let result = execute(
            &mut engine,
            &parse("SECTION emp AT id=0 PROJECT (name, salary)").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2); // only name + salary
            }
            _ => panic!("Expected rows"),
        }

        // EXISTS SECTION
        let result = execute(&mut engine, &parse("EXISTS SECTION emp AT id=0").unwrap()).unwrap();
        assert_eq!(result, ExecResult::Bool(true));
        let result = execute(&mut engine, &parse("EXISTS SECTION emp AT id=999").unwrap()).unwrap();
        assert_eq!(result, ExecResult::Bool(false));

        // COVER ALL
        let result = execute(&mut engine, &parse("COVER emp ALL").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // COVER ON (bitmap query)
        let result = execute(&mut engine, &parse("COVER emp ON dept = 'Eng'").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 5),
            _ => panic!("Expected rows"),
        }

        // COVER WHERE (filter query)
        let result = execute(
            &mut engine,
            &parse("COVER emp WHERE salary > 70000").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 2), // 80000, 90000
            _ => panic!("Expected rows"),
        }

        // COVER DISTINCT
        let result = execute(&mut engine, &parse("COVER emp DISTINCT dept").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 1), // just "Eng"
            _ => panic!("Expected rows"),
        }

        // REDEFINE (update)
        execute(
            &mut engine,
            &parse("REDEFINE emp AT id=0 SET (salary: 99000)").unwrap(),
        )
        .unwrap();
        let result = execute(&mut engine, &parse("SECTION emp AT id=0").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(
                    rows[0].get("salary"),
                    Some(&crate::types::Value::Integer(99000))
                );
            }
            _ => panic!("Expected rows"),
        }

        // RETRACT (delete)
        execute(&mut engine, &parse("RETRACT emp AT id=4").unwrap()).unwrap();
        let result = execute(&mut engine, &parse("COVER emp ALL").unwrap()).unwrap();
        match result {
            ExecResult::Rows(rows) => assert_eq!(rows.len(), 4),
            _ => panic!("Expected rows"),
        }

        // INTEGRATE (aggregation)
        let result = execute(
            &mut engine,
            &parse("INTEGRATE emp OVER dept MEASURE avg(salary), count(*)").unwrap(),
        )
        .unwrap();
        match result {
            ExecResult::Rows(rows) => {
                assert_eq!(rows.len(), 1); // one group: "Eng"
                assert!(rows[0].contains_key("dept"));
            }
            _ => panic!("Expected rows"),
        }

        // CURVATURE
        let result = execute(&mut engine, &parse("CURVATURE emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        // SPECTRAL
        let result = execute(&mut engine, &parse("SPECTRAL emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Scalar(_)));

        // SHOW BUNDLES
        let result = execute(&mut engine, &parse("SHOW BUNDLES").unwrap()).unwrap();
        match result {
            ExecResult::Bundles(infos) => {
                assert_eq!(infos.len(), 1);
                assert_eq!(infos[0].name, "emp");
            }
            _ => panic!("Expected Bundles"),
        }

        // DESCRIBE
        let result = execute(&mut engine, &parse("DESCRIBE emp").unwrap()).unwrap();
        match result {
            ExecResult::Stats(stats) => {
                assert_eq!(stats.record_count, 4);
                assert_eq!(stats.base_fields, 1);
                assert_eq!(stats.fiber_fields, 3);
            }
            _ => panic!("Expected Stats"),
        }

        // HEALTH
        let result = execute(&mut engine, &parse("HEALTH emp").unwrap()).unwrap();
        assert!(matches!(result, ExecResult::Stats(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GQL v2.1 tests ──

    #[test]
    fn gql_weave_role() {
        let stmt = parse("WEAVE ROLE analyst PASSWORD 'hash123' INHERITS reader").unwrap();
        match stmt {
            Statement::WeaveRole {
                name,
                password,
                inherits,
                superweave,
            } => {
                assert_eq!(name, "analyst");
                assert_eq!(password, Some("hash123".into()));
                assert_eq!(inherits, Some("reader".into()));
                assert!(!superweave);
            }
            _ => panic!("Expected WeaveRole"),
        }
    }

    #[test]
    fn gql_weave_role_superweave() {
        let stmt = parse("WEAVE ROLE admin SUPERWEAVE").unwrap();
        match stmt {
            Statement::WeaveRole {
                name, superweave, ..
            } => {
                assert_eq!(name, "admin");
                assert!(superweave);
            }
            _ => panic!("Expected WeaveRole"),
        }
    }

    #[test]
    fn gql_unweave_role() {
        let stmt = parse("UNWEAVE ROLE analyst").unwrap();
        match stmt {
            Statement::UnweaveRole { name } => assert_eq!(name, "analyst"),
            _ => panic!("Expected UnweaveRole"),
        }
    }

    #[test]
    fn gql_show_roles() {
        assert!(matches!(parse("SHOW ROLES").unwrap(), Statement::ShowRoles));
    }

    #[test]
    fn gql_grant() {
        let stmt = parse("GRANT COVER, INTEGRATE ON sensors TO analyst").unwrap();
        match stmt {
            Statement::Grant {
                operations,
                bundle,
                role,
            } => {
                assert_eq!(operations, vec!["COVER", "INTEGRATE"]);
                assert_eq!(bundle, "sensors");
                assert_eq!(role, "analyst");
            }
            _ => panic!("Expected Grant"),
        }
    }

    #[test]
    fn gql_revoke() {
        let stmt = parse("REVOKE RETRACT ON sensors FROM analyst").unwrap();
        match stmt {
            Statement::Revoke {
                operations,
                bundle,
                role,
            } => {
                assert_eq!(operations, vec!["RETRACT"]);
                assert_eq!(bundle, "sensors");
                assert_eq!(role, "analyst");
            }
            _ => panic!("Expected Revoke"),
        }
    }

    #[test]
    fn gql_drop_policy() {
        let stmt = parse("DROP POLICY region_restrict ON sensors").unwrap();
        match stmt {
            Statement::DropPolicy { name, bundle } => {
                assert_eq!(name, "region_restrict");
                assert_eq!(bundle, "sensors");
            }
            _ => panic!("Expected DropPolicy"),
        }
    }

    #[test]
    fn gql_audit_on() {
        let stmt = parse("AUDIT sensors ON").unwrap();
        match stmt {
            Statement::AuditOn { bundle, operations } => {
                assert_eq!(bundle, "sensors");
                assert!(operations.is_empty());
            }
            _ => panic!("Expected AuditOn"),
        }
    }

    #[test]
    fn gql_audit_off() {
        let stmt = parse("AUDIT sensors OFF").unwrap();
        match stmt {
            Statement::AuditOff { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected AuditOff"),
        }
    }

    #[test]
    fn gql_audit_show() {
        let stmt = parse("AUDIT SHOW sensors SINCE '2024-01-01' ROLE admin").unwrap();
        match stmt {
            Statement::AuditShow {
                bundle,
                since,
                role,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(since, Some("2024-01-01".into()));
                assert_eq!(role, Some("admin".into()));
            }
            _ => panic!("Expected AuditShow"),
        }
    }

    #[test]
    fn gql_gauge_constrain() {
        let stmt =
            parse("GAUGE orders CONSTRAIN (ADD CHECK (total > 0) AS positive_total)").unwrap();
        match stmt {
            Statement::GaugeConstrain {
                bundle,
                constraints,
            } => {
                assert_eq!(bundle, "orders");
                assert_eq!(constraints.len(), 1);
                assert!(constraints[0].contains("CHECK"));
            }
            _ => panic!("Expected GaugeConstrain"),
        }
    }

    #[test]
    fn gql_gauge_unconstrain() {
        let stmt = parse("GAUGE orders UNCONSTRAIN positive_total").unwrap();
        match stmt {
            Statement::GaugeUnconstrain {
                bundle,
                constraint_name,
            } => {
                assert_eq!(bundle, "orders");
                assert_eq!(constraint_name, "positive_total");
            }
            _ => panic!("Expected GaugeUnconstrain"),
        }
    }

    #[test]
    fn gql_show_constraints() {
        let stmt = parse("SHOW CONSTRAINTS ON sensors").unwrap();
        match stmt {
            Statement::ShowConstraints { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowConstraints"),
        }
    }

    #[test]
    fn gql_compact() {
        let stmt = parse("COMPACT sensors ANALYZE").unwrap();
        match stmt {
            Statement::Compact { bundle, analyze } => {
                assert_eq!(bundle, "sensors");
                assert!(analyze);
            }
            _ => panic!("Expected Compact"),
        }
    }

    #[test]
    fn gql_analyze() {
        let stmt = parse("ANALYZE sensors FULL").unwrap();
        match stmt {
            Statement::Analyze {
                bundle,
                field,
                full,
            } => {
                assert_eq!(bundle, "sensors");
                assert!(field.is_none());
                assert!(full);
            }
            _ => panic!("Expected Analyze"),
        }
    }

    #[test]
    fn gql_analyze_field() {
        let stmt = parse("ANALYZE sensors ON temp").unwrap();
        match stmt {
            Statement::Analyze {
                bundle,
                field,
                full,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, Some("temp".into()));
                assert!(!full);
            }
            _ => panic!("Expected Analyze"),
        }
    }

    #[test]
    fn gql_vacuum() {
        let stmt = parse("VACUUM sensors FULL").unwrap();
        match stmt {
            Statement::Vacuum { bundle, full } => {
                assert_eq!(bundle, "sensors");
                assert!(full);
            }
            _ => panic!("Expected Vacuum"),
        }
    }

    #[test]
    fn gql_rebuild_index() {
        let stmt = parse("REBUILD INDEX sensors ON city").unwrap();
        match stmt {
            Statement::RebuildIndex { bundle, field } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, Some("city".into()));
            }
            _ => panic!("Expected RebuildIndex"),
        }
    }

    #[test]
    fn gql_check_integrity() {
        let stmt = parse("CHECK sensors").unwrap();
        match stmt {
            Statement::CheckIntegrity { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected CheckIntegrity"),
        }
    }

    #[test]
    fn gql_repair() {
        let stmt = parse("REPAIR sensors").unwrap();
        match stmt {
            Statement::Repair { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected Repair"),
        }
    }

    #[test]
    fn gql_storage() {
        let stmt = parse("STORAGE sensors").unwrap();
        match stmt {
            Statement::StorageInfo { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected StorageInfo"),
        }
    }

    #[test]
    fn gql_set() {
        let stmt = parse("SET TOLERANCE 0.01").unwrap();
        match stmt {
            Statement::Set { key, value } => {
                assert_eq!(key, "TOLERANCE");
                assert_eq!(value, Literal::Float(0.01));
            }
            _ => panic!("Expected Set"),
        }
    }

    #[test]
    fn gql_reset() {
        assert!(matches!(
            parse("RESET ALL").unwrap(),
            Statement::Reset { key: None }
        ));
        let stmt = parse("RESET TOLERANCE").unwrap();
        match stmt {
            Statement::Reset { key } => assert_eq!(key, Some("TOLERANCE".into())),
            _ => panic!("Expected Reset"),
        }
    }

    #[test]
    fn gql_show_settings() {
        assert!(matches!(
            parse("SHOW SETTINGS").unwrap(),
            Statement::ShowSettings
        ));
    }

    #[test]
    fn gql_show_session() {
        assert!(matches!(
            parse("SHOW SESSION").unwrap(),
            Statement::ShowSession
        ));
    }

    #[test]
    fn gql_show_current_role() {
        let stmt = parse("SHOW CURRENT ROLE").unwrap();
        assert!(matches!(stmt, Statement::ShowCurrentRole));
    }

    #[test]
    fn gql_ingest() {
        let stmt = parse("INGEST sensors FROM 'data.csv' FORMAT CSV").unwrap();
        match stmt {
            Statement::Ingest {
                bundle,
                source,
                format,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(source, "data.csv");
                assert_eq!(format, "CSV");
            }
            _ => panic!("Expected Ingest"),
        }
    }

    #[test]
    fn gql_ingest_stdin() {
        let stmt = parse("INGEST sensors FROM STDIN FORMAT JSONL").unwrap();
        match stmt {
            Statement::Ingest { source, format, .. } => {
                assert_eq!(source, "STDIN");
                assert_eq!(format, "JSONL");
            }
            _ => panic!("Expected Ingest"),
        }
    }

    #[test]
    fn gql_transplant() {
        let stmt =
            parse("TRANSPLANT sensors INTO sensors_archive WHERE date < 20240101 RETRACT SOURCE")
                .unwrap();
        match stmt {
            Statement::Transplant {
                source,
                target,
                conditions,
                retract_source,
            } => {
                assert_eq!(source, "sensors");
                assert_eq!(target, "sensors_archive");
                assert_eq!(conditions.len(), 1);
                assert!(retract_source);
            }
            _ => panic!("Expected Transplant"),
        }
    }

    #[test]
    fn gql_generate_base() {
        let stmt =
            parse("GENERATE BASE sensors FROM date=20240101 TO date=20241231 STEP 1").unwrap();
        match stmt {
            Statement::GenerateBase {
                bundle,
                field,
                from_val,
                to_val,
                step,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, "date");
                assert_eq!(from_val, Literal::Integer(20240101));
                assert_eq!(to_val, Literal::Integer(20241231));
                assert_eq!(step, Literal::Integer(1));
            }
            _ => panic!("Expected GenerateBase"),
        }
    }

    #[test]
    fn gql_fill() {
        let stmt = parse("FILL sensors ON date USING INTERPOLATE LINEAR").unwrap();
        match stmt {
            Statement::Fill {
                bundle,
                field,
                method,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(field, "date");
                assert_eq!(method, "INTERPOLATE LINEAR");
            }
            _ => panic!("Expected Fill"),
        }
    }

    #[test]
    fn gql_prepare() {
        let stmt =
            parse("PREPARE city_query AS COVER sensors ON city = $1 WHERE temp < $2").unwrap();
        match stmt {
            Statement::Prepare { name, body } => {
                assert_eq!(name, "city_query");
                assert!(body.contains("COVER"));
                assert!(body.contains("sensors"));
            }
            _ => panic!("Expected Prepare"),
        }
    }

    #[test]
    fn gql_execute_params() {
        let stmt = parse("EXECUTE city_query ('Moscow', -25)").unwrap();
        match stmt {
            Statement::Execute { name, params } => {
                assert_eq!(name, "city_query");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], Literal::Text("Moscow".into()));
                assert_eq!(params[1], Literal::Integer(-25));
            }
            _ => panic!("Expected Execute"),
        }
    }

    #[test]
    fn gql_deallocate() {
        assert!(matches!(
            parse("DEALLOCATE ALL").unwrap(),
            Statement::Deallocate { name: None }
        ));
        let stmt = parse("DEALLOCATE city_query").unwrap();
        match stmt {
            Statement::Deallocate { name } => assert_eq!(name, Some("city_query".into())),
            _ => panic!("Expected Deallocate"),
        }
    }

    #[test]
    fn gql_show_prepared() {
        assert!(matches!(
            parse("SHOW PREPARED").unwrap(),
            Statement::ShowPrepared
        ));
    }

    #[test]
    fn gql_backup() {
        let stmt = parse("BACKUP sensors TO 'sensors_2024.gigi' COMPRESS").unwrap();
        match stmt {
            Statement::Backup {
                bundle,
                path,
                compress,
                incremental_since,
            } => {
                assert_eq!(bundle, Some("sensors".into()));
                assert_eq!(path, "sensors_2024.gigi");
                assert!(compress);
                assert!(incremental_since.is_none());
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_backup_all() {
        let stmt = parse("BACKUP ALL TO 'full.gigi'").unwrap();
        match stmt {
            Statement::Backup { bundle, path, .. } => {
                assert!(bundle.is_none());
                assert_eq!(path, "full.gigi");
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_backup_incremental() {
        let stmt = parse("BACKUP sensors TO 'incr.gigi' INCREMENTAL SINCE '2024-06-01'").unwrap();
        match stmt {
            Statement::Backup {
                incremental_since, ..
            } => {
                assert_eq!(incremental_since, Some("2024-06-01".into()));
            }
            _ => panic!("Expected Backup"),
        }
    }

    #[test]
    fn gql_restore() {
        let stmt = parse("RESTORE sensors FROM 'sensors_2024.gigi' AS sensors_restored").unwrap();
        match stmt {
            Statement::Restore {
                bundle,
                path,
                snapshot,
                rename,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(path, "sensors_2024.gigi");
                assert!(snapshot.is_none());
                assert_eq!(rename, Some("sensors_restored".into()));
            }
            _ => panic!("Expected Restore"),
        }
    }

    #[test]
    fn gql_restore_snapshot() {
        let stmt = parse("RESTORE sensors FROM 'backup.gigi' AT SNAPSHOT 'pre_migration'").unwrap();
        match stmt {
            Statement::Restore { snapshot, .. } => {
                assert_eq!(snapshot, Some("pre_migration".into()));
            }
            _ => panic!("Expected Restore"),
        }
    }

    #[test]
    fn gql_verify_backup() {
        let stmt = parse("VERIFY BACKUP 'sensors_2024.gigi'").unwrap();
        match stmt {
            Statement::VerifyBackup { path } => assert_eq!(path, "sensors_2024.gigi"),
            _ => panic!("Expected VerifyBackup"),
        }
    }

    #[test]
    fn gql_show_backups() {
        assert!(matches!(
            parse("SHOW BACKUPS").unwrap(),
            Statement::ShowBackups
        ));
    }

    #[test]
    fn gql_show_fields() {
        let stmt = parse("SHOW FIELDS ON sensors").unwrap();
        match stmt {
            Statement::ShowFields { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowFields"),
        }
    }

    #[test]
    fn gql_show_indexes() {
        let stmt = parse("SHOW INDEXES ON sensors").unwrap();
        match stmt {
            Statement::ShowIndexes { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowIndexes"),
        }
    }

    #[test]
    fn gql_show_morphisms() {
        let stmt = parse("SHOW MORPHISMS ON orders").unwrap();
        match stmt {
            Statement::ShowMorphisms { bundle } => assert_eq!(bundle, "orders"),
            _ => panic!("Expected ShowMorphisms"),
        }
    }

    #[test]
    fn gql_show_triggers() {
        let stmt = parse("SHOW TRIGGERS ON sensors").unwrap();
        match stmt {
            Statement::ShowTriggers { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowTriggers"),
        }
    }

    #[test]
    fn gql_show_policies() {
        let stmt = parse("SHOW POLICIES ON sensors").unwrap();
        match stmt {
            Statement::ShowPolicies { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowPolicies"),
        }
    }

    #[test]
    fn gql_show_statistics() {
        let stmt = parse("SHOW STATISTICS ON sensors").unwrap();
        match stmt {
            Statement::ShowStatistics { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowStatistics"),
        }
    }

    #[test]
    fn gql_show_geometry() {
        let stmt = parse("SHOW GEOMETRY ON sensors").unwrap();
        match stmt {
            Statement::ShowGeometry { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowGeometry"),
        }
    }

    #[test]
    fn gql_show_comments() {
        let stmt = parse("SHOW COMMENTS ON sensors").unwrap();
        match stmt {
            Statement::ShowComments { bundle } => assert_eq!(bundle, "sensors"),
            _ => panic!("Expected ShowComments"),
        }
    }

    #[test]
    fn gql_comment_on_bundle() {
        let stmt = parse("COMMENT ON BUNDLE sensors IS 'NASA atmospheric data'").unwrap();
        match stmt {
            Statement::CommentOn {
                target_type,
                target,
                comment,
            } => {
                assert_eq!(target_type, "BUNDLE");
                assert_eq!(target, "sensors");
                assert_eq!(comment, "NASA atmospheric data");
            }
            _ => panic!("Expected CommentOn"),
        }
    }

    #[test]
    fn gql_comment_on_field() {
        let stmt = parse("COMMENT ON FIELD sensors.temp IS 'Temperature at 2m'").unwrap();
        match stmt {
            Statement::CommentOn {
                target_type,
                target,
                comment,
            } => {
                assert_eq!(target_type, "FIELD");
                assert_eq!(target, "sensors.temp");
                assert_eq!(comment, "Temperature at 2m");
            }
            _ => panic!("Expected CommentOn"),
        }
    }

    #[test]
    fn gql_iterate() {
        let stmt =
            parse("ITERATE employees START AT id=1 STEP ALONG manager_id UNTIL VOID MAX DEPTH 10")
                .unwrap();
        match stmt {
            Statement::Iterate {
                bundle,
                start_key,
                step_field,
                max_depth,
            } => {
                assert_eq!(bundle, "employees");
                assert_eq!(start_key, vec![("id".into(), Literal::Integer(1))]);
                assert_eq!(step_field, "manager_id");
                assert_eq!(max_depth, Some(10));
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_iterate_no_depth() {
        let stmt =
            parse("ITERATE friends START AT user_id=42 STEP ALONG friend_id UNTIL VOID").unwrap();
        match stmt {
            Statement::Iterate {
                bundle,
                step_field,
                max_depth,
                ..
            } => {
                assert_eq!(bundle, "friends");
                assert_eq!(step_field, "friend_id");
                assert!(max_depth.is_none());
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_iterate_depth_only() {
        let stmt = parse("ITERATE friends START AT user_id=42 STEP ALONG friend_id UNTIL DEPTH 3")
            .unwrap();
        match stmt {
            Statement::Iterate { max_depth, .. } => {
                assert_eq!(max_depth, Some(3));
            }
            _ => panic!("Expected Iterate"),
        }
    }

    #[test]
    fn gql_drop_trigger() {
        let stmt = parse("DROP TRIGGER extreme_cold ON sensors").unwrap();
        match stmt {
            Statement::DropTrigger { name, bundle } => {
                assert_eq!(name, "extreme_cold");
                assert_eq!(bundle, "sensors");
            }
            _ => panic!("Expected DropTrigger"),
        }
    }

    #[test]
    fn gql_on_trigger() {
        let stmt = parse("ON SECTION sensors EXECUTE NOTIFY 'new_reading'").unwrap();
        match stmt {
            Statement::CreateTrigger {
                event,
                bundle,
                condition,
                action,
            } => {
                assert_eq!(event, "ON SECTION");
                assert_eq!(bundle, "sensors");
                assert!(condition.is_none());
                assert!(action.contains("NOTIFY"));
            }
            _ => panic!("Expected CreateTrigger"),
        }
    }

    #[test]
    fn gql_on_trigger_with_condition() {
        let stmt =
            parse("ON SECTION sensors WHERE temp < -30 EXECUTE ALERT 'extreme_cold'").unwrap();
        match stmt {
            Statement::CreateTrigger {
                event,
                bundle,
                condition,
                action,
            } => {
                assert_eq!(event, "ON SECTION");
                assert_eq!(bundle, "sensors");
                assert!(condition.is_some());
                assert!(condition.unwrap().contains("temp"));
                assert!(action.contains("ALERT"));
            }
            _ => panic!("Expected CreateTrigger"),
        }
    }

    #[test]
    fn gql_sections_column_list_tuples() {
        let stmt =
            parse("SECTIONS sensors (id, city, temp) (1, 'Moscow', -27.1), (2, 'Berlin', 5.0)")
                .unwrap();
        match stmt {
            Statement::BatchInsert {
                bundle,
                columns,
                rows,
            } => {
                assert_eq!(bundle, "sensors");
                assert_eq!(columns, vec!["id", "city", "temp"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 3);
                assert_eq!(rows[1].len(), 3);
            }
            _ => panic!("Expected BatchInsert"),
        }
    }

    #[test]
    fn gql_sections_column_list_single_tuple() {
        let stmt = parse("SECTIONS s (a, b) (1, 'x')").unwrap();
        match stmt {
            Statement::BatchInsert {
                bundle,
                columns,
                rows,
            } => {
                assert_eq!(bundle, "s");
                assert_eq!(columns, vec!["a", "b"]);
                assert_eq!(rows.len(), 1);
            }
            _ => panic!("Expected BatchInsert"),
        }
    }
}
