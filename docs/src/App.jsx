import { useState, useEffect, useRef } from "react";

// ═══════════════════════════════════════════════════════════════════
// GQL REFERENCE — Comprehensive Documentation
// Parity target: PostgreSQL / MySQL official docs
// ═══════════════════════════════════════════════════════════════════

const G = "#40E8A0";
const GD = "#2BA070";
const BG = "#06060A";
const CARD = "#0C0E16";
const BORDER = "rgba(255,255,255,0.06)";
const MONO = "'SF Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace";

// ─── Navigation structure ────────────────────────────────────────
const NAV = [
  { id: "overview", label: "Overview", icon: "📖" },
  { id: "quickstart", label: "Quick Start", icon: "⚡" },
  { id: "concepts", label: "Core Concepts", icon: "🧬" },
  { id: "types", label: "Data Types", icon: "🔢" },
  { id: "bundles", label: "Bundles (DDL)", icon: "📦" },
  { id: "sections", label: "Sections (DML)", icon: "✏️" },
  { id: "queries", label: "Queries", icon: "🔍" },
  { id: "filters", label: "Filters & Operators", icon: "⚙️" },
  { id: "aggregation", label: "Aggregation", icon: "📊" },
  { id: "joins", label: "Joins (Pullback)", icon: "🔗" },
  { id: "transactions", label: "Transactions", icon: "🔒" },
  { id: "geometric", label: "Geometric Analytics", icon: "📐" },
  { id: "sql", label: "SQL Compatibility", icon: "🗄️" },
  { id: "access", label: "Access Control", icon: "🛡️" },
  { id: "constraints", label: "Constraints", icon: "📏" },
  { id: "indexes", label: "Indexes", icon: "📑" },
  { id: "prepared", label: "Prepared Statements", icon: "📋" },
  { id: "triggers", label: "Triggers", icon: "⚡" },
  { id: "maintenance", label: "Maintenance", icon: "🔧" },
  { id: "backup", label: "Backup & Restore", icon: "💾" },
  { id: "import", label: "Import & Export", icon: "📤" },
  { id: "encryption", label: "Encryption", icon: "🔐" },
  { id: "rest", label: "REST API", icon: "🌐" },
  { id: "websocket", label: "WebSocket", icon: "📡" },
  { id: "sdk", label: "JavaScript SDK", icon: "📦" },
  { id: "edge", label: "Edge Sync", icon: "🌍" },
  { id: "config", label: "Configuration", icon: "⚙️" },
  { id: "functions", label: "Functions", icon: "ƒ" },
  { id: "reserved", label: "Reserved Words", icon: "📝" },
  { id: "math", label: "Mathematical Reference", icon: "∑" },
  { id: "errors", label: "Error Reference", icon: "⚠️" },
  { id: "glossary", label: "Glossary", icon: "📚" },
];

// ─── Code Block Component ────────────────────────────────────────
function Code({ children, title, lang }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(children);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div style={{ margin: "12px 0", borderRadius: 8, border: `1px solid ${BORDER}`, overflow: "hidden" }}>
      {title && (
        <div style={{ padding: "6px 14px", background: "#0A0C14", borderBottom: `1px solid ${BORDER}`, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 11, color: "#606878", fontFamily: MONO }}>{title}</span>
          <button onClick={copy} style={{ background: "none", border: "none", color: copied ? G : "#404860", fontSize: 11, cursor: "pointer", fontFamily: MONO }}>{copied ? "✓ copied" : "copy"}</button>
        </div>
      )}
      <pre style={{ padding: "14px 16px", background: "#080A12", margin: 0, overflow: "auto", fontSize: 13, lineHeight: 1.6, fontFamily: MONO, color: "#B8C4D0" }}>
        <code>{children}</code>
      </pre>
    </div>
  );
}

// ─── Table Component ─────────────────────────────────────────────
function Table({ headers, rows }) {
  return (
    <div style={{ overflow: "auto", margin: "12px 0", borderRadius: 8, border: `1px solid ${BORDER}` }}>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
        <thead>
          <tr style={{ background: "#0A0C14" }}>
            {headers.map((h, i) => <th key={i} style={{ padding: "10px 14px", textAlign: "left", color: G, fontWeight: 600, fontSize: 11, letterSpacing: "0.05em", borderBottom: `1px solid ${BORDER}`, fontFamily: MONO, whiteSpace: "nowrap" }}>{h}</th>)}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, ri) => (
            <tr key={ri} style={{ background: ri % 2 === 0 ? "#080A12" : "#0B0D16" }}>
              {row.map((cell, ci) => <td key={ci} style={{ padding: "8px 14px", borderBottom: `1px solid ${BORDER}`, color: ci === 0 ? "#D0D8E4" : "#8890A0", fontFamily: ci === 0 ? MONO : "inherit", fontSize: 13 }}>{cell}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ─── Section Component ───────────────────────────────────────────
function Section({ id, title, children }) {
  return (
    <section id={id} style={{ marginBottom: 56 }}>
      <h2 style={{ fontSize: 26, fontWeight: 800, color: "#E0E8F0", marginBottom: 20, paddingBottom: 12, borderBottom: `2px solid ${G}22`, letterSpacing: "-0.02em" }}>{title}</h2>
      {children}
    </section>
  );
}

function H3({ children, id }) {
  return <h3 id={id} style={{ fontSize: 18, fontWeight: 700, color: "#D0D8E4", margin: "32px 0 12px", paddingBottom: 8, borderBottom: `1px solid ${BORDER}` }}>{children}</h3>;
}

function H4({ children }) {
  return <h4 style={{ fontSize: 14, fontWeight: 700, color: G, margin: "20px 0 8px", fontFamily: MONO }}>{children}</h4>;
}

function P({ children }) {
  return <p style={{ fontSize: 14.5, lineHeight: 1.75, color: "#909CAC", margin: "8px 0 12px" }}>{children}</p>;
}

function Note({ type, children }) {
  const colors = { info: ["#3B82F6", "rgba(59,130,246,0.08)"], warn: ["#F59E0B", "rgba(245,158,11,0.08)"], tip: [G, "rgba(64,232,160,0.06)"] };
  const [c, bg] = colors[type] || colors.info;
  const labels = { info: "NOTE", warn: "WARNING", tip: "TIP" };
  return (
    <div style={{ margin: "16px 0", padding: "12px 16px", borderRadius: 8, background: bg, borderLeft: `3px solid ${c}` }}>
      <div style={{ fontSize: 10, fontWeight: 800, color: c, letterSpacing: "0.1em", marginBottom: 6, fontFamily: MONO }}>{labels[type]}</div>
      <div style={{ fontSize: 13.5, lineHeight: 1.7, color: "#A0ACB8" }}>{children}</div>
    </div>
  );
}

function Comparison({ gql, sql }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, margin: "12px 0" }}>
      <Code title="GQL">{gql}</Code>
      <Code title="SQL Equivalent">{sql}</Code>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  DOCUMENTATION SECTIONS
// ═══════════════════════════════════════════════════════════════════

function OverviewSection() {
  return (
    <Section id="overview" title="Overview">
      <P>GQL (Geometric Query Language) is the native query language for GIGI — the Geometric Intrinsic Global Index. It is a fully featured data definition, manipulation, and analytics language built on the mathematics of fiber bundles, gauge theory, and Riemannian geometry.</P>

      <P>Where SQL treats data as flat rows in tables, GQL treats data as <strong style={{ color: "#D0D8E4" }}>sections of a fiber bundle</strong> — a rigorous topological structure where every record has a well-defined geometric position and every query has a provable complexity bound.</P>

      <H3>Key Differences from SQL</H3>
      <Table
        headers={["Concept", "SQL", "GQL", "Why"]}
        rows={[
          ["Table", "CREATE TABLE", "BUNDLE ... BASE (...) FIBER (...)", "Fiber bundle (E, B, F, π) — schema IS geometry"],
          ["Row", "INSERT INTO", "SECTION name (col: val)", "Section σ: B → E of the projection"],
          ["SELECT where", "SELECT ... WHERE", "COVER ... ON/WHERE", "Open cover of the base space"],
          ["Update", "UPDATE ... SET", "REDEFINE ... SET", "Redefine the section at a point"],
          ["Delete", "DELETE FROM", "RETRACT ... AT", "Retract the section (remove fiber)"],
          ["GROUP BY", "SELECT ... GROUP BY", "INTEGRATE ... OVER ... MEASURE", "Integration over the fiber"],
          ["JOIN", "JOIN ... ON", "PULLBACK ... ALONG ... ONTO", "Categorical pullback (exact)"],
          ["Transaction", "BEGIN/COMMIT", "ATLAS BEGIN/COMMIT", "Atlas of charts on the manifold"],
          ["Drop table", "DROP TABLE", "COLLAPSE name", "Collapse the bundle"],
          ["Describe", "DESCRIBE", "DESCRIBE name", "Same keyword, richer output"],
          ["NULL check", "IS NULL / IS NOT NULL", "VOID / DEFINED", "Geometric: undefined vs. defined section"],
          ["LIKE", "LIKE '%pat%'", "MATCHES 'pattern'", "Regex-based pattern matching"],
        ]}
      />

      <H3>Design Principles</H3>
      <P><strong style={{ color: "#D0D8E4" }}>1. Every operation has a geometric meaning.</strong> There are no arbitrary naming choices — BUNDLE, SECTION, COVER, INTEGRATE, PULLBACK, CURVATURE all correspond to mathematical operations on fiber bundles.</P>
      <P><strong style={{ color: "#D0D8E4" }}>2. Complexity is provable.</strong> Point queries are O(1) by construction (section evaluation). Range queries are O(|result|) over indexed fields (open cover). Joins are O(|left|) (pullback along a morphism). These are not amortized estimates — they follow from the data structure itself.</P>
      <P><strong style={{ color: "#D0D8E4" }}>3. The database knows its own quality.</strong> Curvature K measures data variance. Confidence = 1/(1+K). Capacity C = τ/K bounds safe growth. Every write returns these metrics. Anomalies are geometric — detectable without ad-hoc rules.</P>
      <P><strong style={{ color: "#D0D8E4" }}>4. Full SQL compatibility.</strong> GQL includes a complete SQL compatibility layer. SELECT, INSERT INTO, CREATE TABLE, JOIN, GROUP BY, BETWEEN, IN — all work. Migrate gradually.</P>

      <H3>Architecture at a Glance</H3>
      <Table
        headers={["Layer", "Name", "Components"]}
        rows={[
          ["Layer 1", "Base Space B", "Hash map: key → record. O(1) point lookup. GIGI Hash (wyhash-inspired, 64-bit)."],
          ["Layer 2", "Index Topology", "Categorical bitmap indexes on fiber fields. COVER ON = O(|bucket|)."],
          ["Layer 3", "Connection", "Parallel transport, curvature K, capacity C = τ/K, Čech cohomology H¹, holonomy."],
          ["Layer 4", "Encryption", "Gauge transforms: affine on fiber values. Curvature-invariant. Analytics on encrypted data."],
        ]}
      />
    </Section>
  );
}

function QuickStartSection() {
  return (
    <Section id="quickstart" title="Quick Start">
      <H3>1. Create a Bundle</H3>
      <P>A bundle is the fundamental data container (analogous to a table). It has a <strong style={{ color: "#D0D8E4" }}>base space</strong> (key fields) and a <strong style={{ color: "#D0D8E4" }}>fiber</strong> (data fields).</P>
      <Code title="Create a sensor bundle">{`BUNDLE sensors
  BASE (id NUMERIC)
  FIBER (
    city     CATEGORICAL INDEX,
    temp     NUMERIC RANGE 80,
    humidity NUMERIC RANGE 100,
    status   CATEGORICAL DEFAULT 'normal'
  );`}</Code>

      <H3>2. Insert Sections</H3>
      <P>Each record is a "section" — a mapping from the base space to the fiber.</P>
      <Code title="Insert records">{`-- Single insert
SECTION sensors (id: 1, city: 'Moscow', temp: -31.9, humidity: 68, status: 'cold');
SECTION sensors (id: 2, city: 'Tokyo',  temp: 22.1,  humidity: 55, status: 'normal');
SECTION sensors (id: 3, city: 'Nairobi', temp: 28.4, humidity: 72, status: 'normal');

-- Batch insert (column-list style)
SECTIONS sensors (id, city, temp, humidity, status)
  (4, 'London', 12.1, 82, 'normal'),
  (5, 'NYC',    -5.3, 45, 'cold'),
  (6, 'Dubai',  44.2, 20, 'hot');`}</Code>

      <H3>3. Query</H3>
      <Code title="Point query — O(1)">{`SECTION sensors AT id=1;
-- Returns: {id: 1, city: "Moscow", temp: -31.9, humidity: 68, status: "cold"}`}</Code>
      <Code title="Indexed range — O(|result|)">{`COVER sensors ON city = 'Moscow';`}</Code>
      <Code title="Filter scan">{`COVER sensors WHERE temp < 0;`}</Code>
      <Code title="Projection + sort + limit">{`COVER sensors WHERE temp > 20
  PROJECT (city, temp)
  RANK BY temp DESC
  FIRST 5;`}</Code>

      <H3>4. Aggregation</H3>
      <Code title="GROUP BY equivalent">{`INTEGRATE sensors OVER city MEASURE
  count(*) AS total,
  avg(temp) AS avg_temp,
  min(temp) AS coldest,
  max(temp) AS hottest;`}</Code>

      <H3>5. Geometric Analytics</H3>
      <Code title="Curvature — quality metric">{`CURVATURE sensors;
-- Returns: K (scalar curvature), confidence, capacity, per-field breakdown

CURVATURE sensors ON temp;
-- Returns: curvature of the temperature field specifically

CURVATURE sensors ON temp BY city;
-- Returns: curvature of temp grouped by city`}</Code>

      <H3>6. SQL Works Too</H3>
      <Code title="SQL compatibility">{`SELECT * FROM sensors WHERE id = 1;
SELECT city, AVG(temp) FROM sensors GROUP BY city;
INSERT INTO sensors (id, city, temp) VALUES (7, 'Berlin', 8.5);`}</Code>

      <Note type="tip">GQL is a superset of common SQL. You can mix GQL-native and SQL-compat syntax freely in the same session.</Note>
    </Section>
  );
}

function ConceptsSection() {
  return (
    <Section id="concepts" title="Core Concepts">
      <H3>Fiber Bundle Model</H3>
      <P>GIGI models every dataset as a <strong style={{ color: "#D0D8E4" }}>fiber bundle</strong> (E, B, F, π) from differential geometry:</P>
      <Table
        headers={["Component", "Math Symbol", "Database Analog", "Description"]}
        rows={[
          ["Total space", "E", "All records", "The set of all data points"],
          ["Base space", "B", "Key space", "The domain of primary keys, hashed to ℤ₂₆₄"],
          ["Fiber", "F", "Data fields", "The value space at each key — product of all non-key fields"],
          ["Projection", "π: E → B", "Record → Key", "Map from record to its key"],
          ["Section", "σ: B → E", "A record", "A mapping that assigns fiber values to a base point"],
          ["Zero section", "σ₀", "Default values", "Section where every field has its DEFAULT value"],
        ]}
      />

      <H3>Geometric Metrics</H3>
      <P>Every fiber field carries a metric (distance function) determined by its type:</P>
      <Table
        headers={["Type", "Metric", "Formula"]}
        rows={[
          ["NUMERIC", "Normalized difference", "g(a,b) = |a-b| / range(F)"],
          ["CATEGORICAL", "Discrete (0 or 1)", "g(a,b) = 0 if a=b, 1 otherwise"],
          ["ORDERED CAT", "Rank distance", "g(a,b) = |rank(a)-rank(b)| / (|F|-1)"],
          ["TIMESTAMP", "Normalized duration", "g(a,b) = |a-b| / time_scale"],
        ]}
      />
      <P>The product metric on the full fiber is: <strong style={{ color: "#D0D8E4" }}>g_F(v,w) = √(Σ ωᵢ · gᵢ(vᵢ, wᵢ)²)</strong></P>

      <H3>Curvature</H3>
      <P>Scalar curvature K quantifies how much the data "curves" — i.e., how much variation exists relative to the field's range:</P>
      <Code title="Curvature formula">{`K = Var(fiber) / range²

-- K ≈ 0  → flat data (highly uniform, high confidence)
-- K > 0  → curved data (more variance, lower confidence)
-- K spike → anomaly (sudden change in distribution)`}</Code>
      <P><strong style={{ color: "#D0D8E4" }}>Confidence</strong> = 1 / (1 + K). A flat bundle (K=0) has confidence 1.0. As curvature increases, confidence decreases.</P>
      <P><strong style={{ color: "#D0D8E4" }}>Capacity</strong> C = τ/K, where τ is a threshold. Estimates how many more records can be inserted before quality degrades.</P>

      <H3>GIGI Hash</H3>
      <P>All base-space lookups use the GIGI Hash — a wyhash-inspired, 64-bit composite hash with type-canonical encoding and field-rotation mixing. Properties:</P>
      <Table
        headers={["Property", "Value"]}
        rows={[
          ["Bit width", "64-bit (ℤ₂₆₄)"],
          ["Collision probability", "< 2⁻⁶⁴ per pair"],
          ["Birthday bound", "~4.3 × 10⁹ records (handled via secondary collision map)"],
          ["Point lookup", "O(1) guaranteed"],
          ["Deterministic", "Same key → same hash, always"],
        ]}
      />

      <H3>Čech Cohomology (Consistency)</H3>
      <P>GIGI uses Čech cohomology to detect data inconsistencies. The first cohomology group H¹ measures "obstructions to gluing" — if H¹ = 0, all overlapping views agree. If H¹ &gt; 0, there are conflicts.</P>
      <Code>{`CONSISTENCY sensors;
-- Returns: h1 (cohomology dimension), cocycles (conflict count)
-- h1 = 0 → perfectly consistent
-- h1 > 0 → conflicts detected

CONSISTENCY sensors REPAIR;
-- Auto-resolve conflicts`}</Code>

      <H3>Holonomy</H3>
      <P>Holonomy measures what happens when you "parallel-transport" a value around a loop in the data graph. Nonzero holonomy means the round-trip doesn't return to the start — a sign of drift, inconsistency, or data quality issues.</P>
    </Section>
  );
}

function TypesSection() {
  return (
    <Section id="types" title="Data Types">
      <H3>Field Types</H3>
      <Table
        headers={["Type", "Aliases", "Description", "Storage", "Metric"]}
        rows={[
          ["NUMERIC", "NUMBER, FLOAT, INT, INTEGER", "64-bit signed integer or IEEE 754 float", "8 bytes", "Normalized difference"],
          ["TEXT", "CATEGORICAL, VARCHAR, STRING", "UTF-8 string of arbitrary length", "Variable", "Discrete (0/1)"],
          ["TIMESTAMP", "TIME, DATE", "Epoch milliseconds (i64)", "8 bytes", "Normalized duration"],
          ["BOOLEAN", "BOOL", "true or false", "Categorical", "Discrete (0/1)"],
        ]}
      />
      <Note type="info">GIGI stores BOOLEAN as a categorical field internally. The keywords TRUE and FALSE are parsed as text literals "true" and "false".</Note>

      <H3>Literal Syntax</H3>
      <Table
        headers={["Type", "Examples", "Notes"]}
        rows={[
          ["Integer", "42, -7, 0, 1000000", "64-bit signed, no separators"],
          ["Float", "3.14, -0.001, 2.0", "IEEE 754 double precision"],
          ["String", "'hello', 'New York', 'it''s'", "Single-quoted, escape quote by doubling"],
          ["Boolean", "true, false", "Case-insensitive"],
          ["Null", "NULL", "Explicit null / undefined"],
          ["Parameter", "$1, $2, $3", "Positional parameter (prepared statements)"],
        ]}
      />

      <H3>Value Representation</H3>
      <Code title="Internal value enum">{`Value::Integer(i64)     -- 42, -7
Value::Float(f64)       -- 3.14, -0.001
Value::Text(String)     -- "hello"
Value::Bool(bool)       -- true, false
Value::Timestamp(i64)   -- epoch milliseconds
Value::Null             -- absence of value`}</Code>

      <H3>Type Coercion</H3>
      <P>GQL performs automatic type coercion in comparisons:</P>
      <Table
        headers={["From", "To", "Rule"]}
        rows={[
          ["Integer", "Float", "Widened automatically in arithmetic"],
          ["String numeric", "Numeric", "Parsed when compared with a NUMERIC field"],
          ["Boolean", "Text", "Stored as 'true' / 'false' strings"],
        ]}
      />

      <H3>Type Casting (Spec)</H3>
      <Code>{`-- CAST syntax
CAST(value AS NUMERIC)
CAST(value AS TEXT)

-- Shorthand
value::NUMERIC
value::TEXT`}</Code>
    </Section>
  );
}

function BundlesSection() {
  return (
    <Section id="bundles" title="Bundles (DDL)">
      <P>Bundles are the fundamental schema objects in GIGI — equivalent to tables in SQL. A bundle defines a fiber bundle (E, B, F, π) with a base space (keys) and fiber (data fields).</P>

      <H3>CREATE BUNDLE</H3>
      <H4>Geometric Syntax (Native GQL)</H4>
      <Code title="Full syntax">{`BUNDLE <name>
  BASE (<field> <type> [, ...])
  FIBER (
    <field> <type> [INDEX] [RANGE <n>] [DEFAULT <val>] [AUTO] [UNIQUE] [REQUIRED]
    [, ...]
  ) [ENCRYPTED];`}</Code>
      <Code title="Example">{`BUNDLE sensors
  BASE (id NUMERIC)
  FIBER (
    city     CATEGORICAL INDEX,
    region   CATEGORICAL INDEX,
    temp     NUMERIC RANGE 80,
    humidity NUMERIC RANGE 100,
    pressure NUMERIC RANGE 50,
    status   CATEGORICAL DEFAULT 'normal',
    reading  NUMERIC AUTO
  );`}</Code>

      <H4>SQL-Compatible Syntax</H4>
      <Code>{`CREATE BUNDLE sensors (
  id       NUMERIC   BASE,
  city     TEXT      FIBER INDEX,
  temp     NUMERIC   FIBER RANGE(80),
  status   TEXT      FIBER DEFAULT 'normal'
);`}</Code>

      <H3>Field Modifiers</H3>
      <Table
        headers={["Modifier", "Syntax", "Description", "SQL Equivalent"]}
        rows={[
          ["INDEX", "field TYPE INDEX", "Creates a bitmap index for O(|bucket|) lookups via COVER ON", "CREATE INDEX"],
          ["RANGE", "field NUMERIC RANGE 80", "Sets the domain width for curvature calculation: K = Var/range²", "—"],
          ["DEFAULT", "field TYPE DEFAULT 'val'", "The zero-section σ₀ value — used when field is omitted on insert", "DEFAULT"],
          ["AUTO", "field NUMERIC AUTO", "Sequential auto-increment, starting from 1", "SERIAL / AUTO_INCREMENT"],
          ["UNIQUE", "field TYPE UNIQUE", "Enforces injectivity — no duplicate values", "UNIQUE"],
          ["REQUIRED", "field TYPE REQUIRED", "Field must be provided on every insert (non-nullable)", "NOT NULL"],
          ["ENCRYPTED", "(bundle-level)", "Enable geometric encryption via gauge transforms on all numeric fields", "—"],
        ]}
      />

      <H3>COLLAPSE (Drop Bundle)</H3>
      <Code>{`COLLAPSE sensors;

-- SQL-compatible alias:
DROP BUNDLE sensors;`}</Code>
      <Note type="warn">COLLAPSE permanently deletes the bundle and all its data. This operation is logged to the WAL and cannot be undone without a backup.</Note>

      <H3>DESCRIBE</H3>
      <Code>{`DESCRIBE sensors;
-- Returns: field names, types, modifiers, base/fiber designation

DESCRIBE sensors VERBOSE;
-- Returns: above + record count, curvature, index info`}</Code>

      <H3>SHOW BUNDLES</H3>
      <Code>{`SHOW BUNDLES;
-- Returns: list of all bundle names in the engine`}</Code>

      <H3>Schema Evolution</H3>
      <Code>{`-- Add a field to an existing bundle
GAUGE sensors TRANSFORM (
  ADD field_name TYPE [DEFAULT value]
);

-- Via REST API:
POST /v1/bundles/sensors/add-field
{ "name": "wind_speed", "field_type": "numeric", "default": 0 }

-- Add an index:
POST /v1/bundles/sensors/add-index
{ "field": "region" }`}</Code>
    </Section>
  );
}

function SectionsSection() {
  return (
    <Section id="sections" title="Sections (DML)">
      <P>Sections are records — each one is a section of the fiber bundle, mapping a base point to fiber values. GQL provides four DML operations: SECTION (insert), REDEFINE (update), RETRACT (delete), and their bulk variants.</P>

      <H3 id="insert">SECTION (Insert)</H3>
      <Code title="Syntax">{`SECTION <bundle> (<field>: <value>, ...);
-- or with = separator:
SECTION <bundle> (<field> = <value>, ...);`}</Code>
      <Code title="Examples">{`SECTION sensors (id: 1, city: 'Moscow', temp: -31.9, humidity: 68);
SECTION sensors (id = 2, city = 'Tokyo', temp = 22.1, humidity = 55);`}</Code>
      <P>Returns curvature metadata after every insert: count, total, curvature K, confidence.</P>

      <H4>SQL-Compatible INSERT</H4>
      <Code>{`INSERT INTO sensors (id, city, temp) VALUES (3, 'Nairobi', 28.4);`}</Code>

      <H3 id="batch-insert">SECTIONS (Batch Insert)</H3>
      <P>Three batch insert patterns for high-throughput ingestion:</P>
      <Code title="Pattern 1: Key-value (single row)">{`SECTIONS sensors (id: 42, city: 'Moscow', temp: -31.9);`}</Code>
      <Code title="Pattern 2: Column-list + value tuples (multi-row)">{`SECTIONS sensors (id, city, temp, status)
  (1, 'Moscow', -31.9, 'cold'),
  (2, 'Tokyo',   22.1, 'normal'),
  (3, 'Nairobi', 28.4, 'normal');`}</Code>
      <Code title="Pattern 3: Positional values">{`SECTIONS sensors (42, 'Moscow', 'EU', 20240104, -31.9);`}</Code>

      <H3 id="upsert">SECTION ... UPSERT</H3>
      <P>Insert or update — if the key exists, overwrite the fiber values.</P>
      <Code>{`SECTION sensors (id: 1, city: 'Moscow', temp: -28.5) UPSERT;`}</Code>

      <H3 id="update">REDEFINE (Update)</H3>
      <H4>Point Update</H4>
      <Code>{`REDEFINE sensors AT id=1 SET (temp: -28.5, status: 'warming');`}</Code>

      <H4>Bulk Update — ON (indexed field)</H4>
      <Code>{`REDEFINE sensors ON city = 'Moscow' SET (region: 'RU');`}</Code>

      <H4>Bulk Update — WHERE (predicate scan)</H4>
      <Code>{`REDEFINE sensors WHERE temp > 35 SET (status: 'hot');
REDEFINE sensors WHERE temp < -20 SET (status: 'extreme_cold');`}</Code>

      <H3 id="delete">RETRACT (Delete)</H3>
      <H4>Point Delete</H4>
      <Code>{`RETRACT sensors AT id=42;`}</Code>

      <H4>Bulk Delete — ON</H4>
      <Code>{`RETRACT sensors ON city = 'TestCity';`}</Code>

      <H4>Bulk Delete — WHERE</H4>
      <Code>{`RETRACT sensors WHERE temp > 100;`}</Code>

      <H3 id="point-query">SECTION AT (Point Query)</H3>
      <P>Point queries evaluate the section at a base point — O(1) via hash lookup.</P>
      <Code>{`-- Simple point query
SECTION sensors AT id=42;

-- With projection
SECTION sensors AT id=42 PROJECT (city, temp, humidity);

-- Composite key
SECTION events AT user_id=1001, timestamp=1710000000;

-- Existence check (returns boolean, no data transfer)
EXISTS SECTION sensors AT id=42;`}</Code>
    </Section>
  );
}

function QueriesSection() {
  return (
    <Section id="queries" title="Queries">
      <P>COVER is the primary query statement in GQL — it returns sections matching conditions over the base and fiber spaces. The name comes from the topological concept of an "open cover" of a space.</P>

      <H3>COVER (Range Query)</H3>
      <Code title="Full syntax">{`COVER <bundle>
  [ALL]
  [ON <field> <op> <value> [, ...]]
  [WHERE <field> <op> <value> [, ...]]
  [OR <field> <op> <value> [, ...]]
  [DISTINCT <field>]
  [PROJECT (<field>, ...)]
  [RANK BY <field> [ASC|DESC] [, ...]]
  [FIRST <n>]
  [SKIP <n>];`}</Code>

      <H3>Clause Reference</H3>
      <Table
        headers={["Clause", "Purpose", "Complexity", "SQL Equivalent"]}
        rows={[
          ["ALL", "Return every record in the bundle", "O(n)", "SELECT *"],
          ["ON", "Indexed field lookup (bitmap index)", "O(|bucket|)", "WHERE (indexed)"],
          ["WHERE", "Fiber predicate scan", "O(n)", "WHERE (unindexed)"],
          ["OR", "Additional OR condition groups", "—", "OR"],
          ["DISTINCT", "Return unique values for a field", "O(n)", "SELECT DISTINCT"],
          ["PROJECT", "Return only specified fields", "—", "SELECT col1, col2"],
          ["RANK BY", "Sort results (ASC or DESC)", "O(n log n)", "ORDER BY"],
          ["FIRST", "Limit result count", "—", "LIMIT"],
          ["SKIP", "Skip first N results (offset)", "—", "OFFSET"],
        ]}
      />

      <H3>Examples</H3>
      <Code title="List all">{`COVER sensors ALL;`}</Code>
      <Code title="Indexed lookup (fast)">{`COVER sensors ON city = 'Moscow';
COVER sensors ON region IN ('EU', 'NA');`}</Code>
      <Code title="Predicate scan">{`COVER sensors WHERE temp < -25;
COVER sensors WHERE temp BETWEEN 10 AND 30;
COVER sensors WHERE status != 'inactive';`}</Code>
      <Code title="Combined (index + filter)">{`COVER sensors ON city = 'Moscow' WHERE temp < -25;`}</Code>
      <Code title="Pattern matching">{`COVER sensors WHERE city MATCHES 'Mos*';`}</Code>
      <Code title="Null checks">{`COVER sensors WHERE pressure VOID;     -- IS NULL
COVER sensors WHERE pressure DEFINED;  -- IS NOT NULL`}</Code>
      <Code title="Distinct values">{`COVER sensors DISTINCT city;`}</Code>
      <Code title="Sorted + paginated">{`COVER sensors RANK BY temp DESC FIRST 10;
COVER sensors RANK BY temp DESC SKIP 10 FIRST 10;  -- page 2`}</Code>
      <Code title="OR conditions">{`COVER sensors WHERE status = 'active' OR status = 'pending';`}</Code>
      <Code title="Full query">{`COVER sensors
  ON region = 'EU'
  WHERE temp > 20
  PROJECT (city, temp, humidity)
  RANK BY temp DESC
  FIRST 5;`}</Code>

      <H3>EXPLAIN</H3>
      <P>Show the query execution plan without running the query:</P>
      <Code>{`EXPLAIN COVER sensors ON city = 'Moscow' WHERE temp < -25;`}</Code>
    </Section>
  );
}

function FiltersSection() {
  return (
    <Section id="filters" title="Filters & Operators">

      <H3>Comparison Operators</H3>
      <Table
        headers={["Operator", "GQL Syntax", "SQL Equivalent", "Example"]}
        rows={[
          ["Equal", "=", "=", "city = 'Moscow'"],
          ["Not equal", "!= or <>", "<> or !=", "status != 'inactive'"],
          ["Greater than", ">", ">", "temp > 30"],
          ["Greater or equal", ">=", ">=", "temp >= 30"],
          ["Less than", "<", "<", "temp < 0"],
          ["Less or equal", "<=", "<=", "temp <= 100"],
          ["In list", "IN (v1, v2)", "IN (v1, v2)", "city IN ('Moscow', 'Tokyo')"],
          ["Not in list", "NOT IN (v1, v2)", "NOT IN", "status NOT IN ('deleted', 'banned')"],
          ["Between", "BETWEEN lo AND hi", "BETWEEN", "temp BETWEEN 10 AND 30"],
          ["Pattern match", "MATCHES 'pat'", "LIKE / ~", "city MATCHES 'Mos*'"],
          ["Contains", "CONTAINS 'text'", "LIKE '%text%'", "name CONTAINS 'smith'"],
          ["Starts with", "starts with", "LIKE 'text%'", "(REST API only)"],
          ["Ends with", "ends with", "LIKE '%text'", "(REST API only)"],
          ["Is null", "VOID", "IS NULL", "pressure VOID"],
          ["Is not null", "DEFINED", "IS NOT NULL", "pressure DEFINED"],
        ]}
      />

      <H3>Logical Operators</H3>
      <P>Multiple conditions in ON/WHERE are combined with AND. Use OR for disjunction:</P>
      <Code>{`-- AND (implicit in comma-separated conditions)
COVER sensors ON city = 'Moscow' WHERE temp < 0;

-- OR
COVER sensors WHERE status = 'active' OR status = 'pending';

-- Complex
COVER sensors
  ON region = 'EU'
  WHERE temp > 20
  OR region = 'NA';`}</Code>

      <H3>REST API Filter Operators</H3>
      <P>When using the REST query endpoint, conditions use a JSON format:</P>
      <Code title="POST /v1/bundles/sensors/query">{`{
  "conditions": [
    { "field": "city", "op": "eq", "value": "Moscow" },
    { "field": "temp", "op": "lt", "value": 0 },
    { "field": "status", "op": "in", "value": ["active", "normal"] }
  ],
  "sort": [{ "field": "temp", "order": "asc" }],
  "limit": 10,
  "offset": 0,
  "fields": ["city", "temp", "status"]
}`}</Code>
      <Table
        headers={["JSON op", "Aliases", "Description"]}
        rows={[
          ["eq", "=, ==", "Equal"],
          ["neq", "!=, <>", "Not equal"],
          ["gt", ">", "Greater than"],
          ["gte", ">=", "Greater or equal"],
          ["lt", "<", "Less than"],
          ["lte", "<=", "Less or equal"],
          ["contains", "like", "Case-insensitive substring"],
          ["starts_with", "startswith", "Prefix match (case-insensitive)"],
          ["ends_with", "endswith", "Suffix match (case-insensitive)"],
          ["regex", "matches", "Regular expression match"],
          ["in", "—", "In array of values"],
          ["not_in", "notin, nin", "Not in array"],
          ["is_null", "isnull", "Field is null"],
          ["is_not_null", "isnotnull, not_null", "Field is not null"],
        ]}
      />
    </Section>
  );
}

function AggregationSection() {
  return (
    <Section id="aggregation" title="Aggregation">
      <P>INTEGRATE is the GQL aggregation statement — analogous to SELECT ... GROUP BY in SQL. The name comes from "integrating over the fiber" — collapsing fiber values into summary statistics.</P>

      <H3>INTEGRATE (GROUP BY)</H3>
      <Code title="Syntax">{`INTEGRATE <bundle>
  [OVER <field>]
  MEASURE <agg>(<field>) [AS <alias>], ...;`}</Code>

      <H3>Aggregate Functions</H3>
      <Table
        headers={["Function", "Syntax", "Description", "Input", "Output"]}
        rows={[
          ["COUNT", "count(*) or count(field)", "Count of non-null values (or all records with *)", "Any", "Integer"],
          ["SUM", "sum(field)", "Sum of numeric values", "Numeric", "Float"],
          ["AVG", "avg(field)", "Arithmetic mean", "Numeric", "Float"],
          ["MIN", "min(field)", "Minimum value", "Any orderable", "Same type"],
          ["MAX", "max(field)", "Maximum value", "Any orderable", "Same type"],
        ]}
      />

      <H3>Examples</H3>
      <Code title="Global aggregation (no grouping)">{`INTEGRATE sensors MEASURE avg(temp), count(*);`}</Code>
      <Code title="Grouped aggregation">{`INTEGRATE sensors OVER city MEASURE
  count(*)    AS total,
  avg(temp)   AS avg_temp,
  min(temp)   AS min_temp,
  max(temp)   AS max_temp,
  sum(humidity) AS total_humidity;`}</Code>
      <Code title="SQL equivalent">{`SELECT city, COUNT(*), AVG(temp), MIN(temp), MAX(temp)
FROM sensors
GROUP BY city;`}</Code>

      <H3>REST API Aggregation</H3>
      <Code title="POST /v1/bundles/sensors/aggregate">{`{
  "group_by": "city",
  "aggregates": ["count", "avg:temp", "min:temp", "max:temp"],
  "conditions": [
    { "field": "status", "op": "eq", "value": "active" }
  ]
}`}</Code>
    </Section>
  );
}

function JoinsSection() {
  return (
    <Section id="joins" title="Joins (Pullback)">
      <P>PULLBACK is the GQL join operation. It implements a categorical pullback — joining records by relationship along a morphism (foreign key). Complexity: O(|left|) via hash lookup into the right bundle.</P>

      <H3>PULLBACK (JOIN)</H3>
      <Code title="Syntax">{`PULLBACK <left_bundle>
  ALONG <left_field>
  ONTO <right_bundle>
  [ALONG <right_field>]
  [PRESERVE LEFT];`}</Code>
      <P>If the second ALONG is omitted, the right bundle's base key is used.</P>

      <H3>Examples</H3>
      <Code title="Simple join">{`-- Join orders to customers via customer_id
PULLBACK orders ALONG customer_id ONTO customers;`}</Code>
      <Code title="Left join (preserve unmatched)">{`PULLBACK orders ALONG customer_id ONTO customers PRESERVE LEFT;`}</Code>
      <Code title="Chain joins">{`-- orders → customers → regions (triple join)
PULLBACK orders
  ALONG customer_id ONTO customers
  ALONG region_id ONTO regions;`}</Code>
      <Comparison
        gql={`PULLBACK orders
  ALONG customer_id
  ONTO customers;`}
        sql={`SELECT *
FROM orders
JOIN customers
  ON orders.customer_id
       = customers.id;`}
      />

      <H3>SQL JOIN Syntax</H3>
      <Code>{`SELECT * FROM orders JOIN customers ON customer_id;`}</Code>

      <H3>REST API Join</H3>
      <Code title="POST /v1/bundles/orders/join">{`{
  "right_bundle": "customers",
  "left_field": "customer_id",
  "right_field": "id"
}`}</Code>

      <Note type="info">PULLBACK is O(|left|) — it iterates the left bundle once and does O(1) lookups into the right bundle for each record. This is always faster than SQL's O(n log n) sort-merge or O(n + m) hash join where both sides must be scanned.</Note>
    </Section>
  );
}

function TransactionsSection() {
  return (
    <Section id="transactions" title="Transactions">
      <P>ATLAS provides transactional semantics in GQL. The name comes from the mathematical concept of an "atlas" — a set of compatible charts covering a manifold. Within an ATLAS block, operations are atomic: either all succeed or all are rolled back.</P>

      <H3>ATLAS (Transaction Block)</H3>
      <Code title="Syntax">{`ATLAS BEGIN;
  <statements>
ATLAS COMMIT;
-- or
ATLAS ROLLBACK;`}</Code>

      <Code title="Example">{`ATLAS BEGIN;
  SECTION accounts (id: 1, balance: 900);
  SECTION accounts (id: 2, balance: 1100);
  REDEFINE audit_log AT id=9001 SET (action: 'transfer');
ATLAS COMMIT;`}</Code>

      <H3>REST API Transaction</H3>
      <Code title="POST /v1/bundles/{name}/transaction">{`{
  "ops": [
    { "type": "insert", "records": [{ "id": 1, "balance": 900 }] },
    { "type": "update", "key": { "id": 2 }, "fields": { "balance": 1100 } },
    { "type": "insert", "records": [{ "event_id": 1, "action": "transfer" }] }
  ]
}`}</Code>
      <P>Supported operation types: <strong style={{ color: "#D0D8E4" }}>insert</strong>, <strong style={{ color: "#D0D8E4" }}>update</strong>, <strong style={{ color: "#D0D8E4" }}>delete</strong>, <strong style={{ color: "#D0D8E4" }}>increment</strong>.</P>

      <H3>Optimistic Concurrency Control</H3>
      <P>Updates and deletes support an <code style={{ fontSize: 12, background: "#0E1020", padding: "2px 6px", borderRadius: 3, fontFamily: MONO }}>expected_version</code> field. If the record has been modified since the specified version, the operation fails with a 409 Conflict.</P>
      <Code>{`// REST update with version check
POST /v1/bundles/accounts/update
{
  "key": { "id": 1 },
  "fields": { "balance": 850 },
  "expected_version": 3
}
// Returns 409 if current version != 3`}</Code>
    </Section>
  );
}

function GeometricSection() {
  return (
    <Section id="geometric" title="Geometric Analytics">
      <P>GIGI provides built-in geometric analytics that have no equivalent in traditional databases. These operations leverage the fiber bundle structure to compute quality metrics, detect anomalies, and verify data consistency — all without external tools or ad-hoc rules.</P>

      <H3>CURVATURE</H3>
      <P>Computes scalar curvature K — a measure of data variance relative to the field's domain range.</P>
      <Code title="Syntax">{`CURVATURE <bundle> [ON <field> [, ...]] [BY <group_field>];`}</Code>
      <Code title="Examples">{`-- Full bundle curvature
CURVATURE sensors;

-- Single field curvature
CURVATURE sensors ON temp;

-- Multi-field curvature
CURVATURE sensors ON temp, humidity, pressure;

-- Curvature grouped by category
CURVATURE sensors ON temp BY city;`}</Code>
      <P>Returns:</P>
      <Table
        headers={["Field", "Description"]}
        rows={[
          ["value (K)", "Scalar curvature — higher means more variance"],
          ["confidence", "1 / (1 + K) — quality metric from 0 to 1"],
          ["capacity", "C = τ/K — estimated remaining safe capacity"],
          ["per_field", "Breakdown of curvature per fiber field"],
        ]}
      />

      <H3>SPECTRAL</H3>
      <P>Spectral analysis via the normalized graph Laplacian — finds the spectral gap λ₁, which measures connectivity and mixing properties.</P>
      <Code>{`SPECTRAL sensors;
-- Returns: lambda_1 (spectral gap), diameter, spectral_capacity

SPECTRAL sensors FULL;
-- Returns: full eigenvalue list`}</Code>
      <P>Spectral gap interpretation:</P>
      <Table
        headers={["λ₁ Value", "Meaning"]}
        rows={[
          ["λ₁ ≈ 0", "Disconnected / clustered data — poor mixing"],
          ["λ₁ large", "Well-connected data — fast random walk convergence"],
        ]}
      />

      <H3>CONSISTENCY</H3>
      <P>Čech cohomology check — verifies that overlapping views of the data agree (sheaf gluing).</P>
      <Code>{`CONSISTENCY sensors;
-- Returns: h1 (cohomology dimension), cocycles (conflict list)
-- h1 = 0 means perfectly consistent

CONSISTENCY sensors REPAIR;
-- Auto-repairs detected inconsistencies`}</Code>

      <H3>HEALTH</H3>
      <P>Full geometric diagnostic — combines curvature, spectral, and consistency into one report.</P>
      <Code>{`HEALTH sensors;`}</Code>

      <H3>Geometric State Functions (Spec)</H3>
      <Table
        headers={["Function", "Returns", "Description"]}
        rows={[
          ["CONFIDENCE()", "Float 0–1", "Current bundle confidence = 1/(1+K)"],
          ["CURVATURE()", "Float", "Current scalar curvature K"],
          ["ANOMALY()", "Boolean", "Whether the last write triggered an anomaly"],
          ["Z_SCORE()", "Float", "Z-score of the last inserted value relative to the field distribution"],
          ["CAPACITY()", "Integer", "Estimated remaining safe capacity C = τ/K"],
        ]}
      />
    </Section>
  );
}

function SQLSection() {
  return (
    <Section id="sql" title="SQL Compatibility">
      <P>GQL includes a full SQL compatibility layer. Standard SQL statements are parsed and translated to their GQL equivalents internally. This enables zero-friction migration from existing SQL databases.</P>

      <H3>SELECT</H3>
      <Code>{`SELECT * FROM sensors WHERE id = 42;
SELECT city, temp FROM sensors WHERE temp > 30;
SELECT * FROM sensors WHERE status IN ('active', 'pending');
SELECT * FROM sensors WHERE temp BETWEEN 10 AND 30;`}</Code>

      <H3>SELECT ... GROUP BY</H3>
      <Code>{`SELECT city, COUNT(*), AVG(temp) FROM sensors GROUP BY city;`}</Code>

      <H3>SELECT ... JOIN</H3>
      <Code>{`SELECT * FROM orders JOIN customers ON customer_id;`}</Code>

      <H3>INSERT INTO</H3>
      <Code>{`INSERT INTO sensors (id, city, temp) VALUES (42, 'Moscow', -31.9);`}</Code>

      <H3>CREATE TABLE → CREATE BUNDLE</H3>
      <Code>{`CREATE BUNDLE sensors (
  id    NUMERIC   BASE,
  city  TEXT      FIBER INDEX,
  temp  NUMERIC   FIBER RANGE(80)
);`}</Code>

      <H3>Mapping Table</H3>
      <Table
        headers={["SQL", "GQL Native", "Notes"]}
        rows={[
          ["SELECT * FROM t WHERE ...", "COVER t ON/WHERE ...", "ON for indexed, WHERE for scan"],
          ["SELECT cols FROM t", "COVER t PROJECT (cols)", ""],
          ["ORDER BY", "RANK BY", ""],
          ["LIMIT n", "FIRST n", ""],
          ["OFFSET n", "SKIP n", ""],
          ["GROUP BY ... AGG()", "INTEGRATE ... OVER ... MEASURE ...", ""],
          ["JOIN ... ON", "PULLBACK ... ALONG ... ONTO", ""],
          ["INSERT INTO", "SECTION", ""],
          ["UPDATE ... SET", "REDEFINE ... SET", ""],
          ["DELETE FROM", "RETRACT", ""],
          ["CREATE TABLE", "BUNDLE ... BASE ... FIBER", ""],
          ["DROP TABLE", "COLLAPSE", ""],
          ["BEGIN/COMMIT", "ATLAS BEGIN/COMMIT", ""],
          ["IS NULL", "VOID", ""],
          ["IS NOT NULL", "DEFINED", ""],
          ["LIKE", "MATCHES", "GQL uses regex, not LIKE globs"],
          ["DESCRIBE", "DESCRIBE", "Same keyword"],
          ["SHOW TABLES", "SHOW BUNDLES", ""],
        ]}
      />
    </Section>
  );
}

function AccessControlSection() {
  return (
    <Section id="access" title="Access Control">
      <P>GQL v2.1 adds role-based access control with row-level security policies, inspired by PostgreSQL's RBAC model but using geometric terminology.</P>

      <H3>Roles</H3>
      <Code title="Create a role">{`WEAVE ROLE analyst PASSWORD 'sha256hash' INHERITS viewer;
WEAVE ROLE admin SUPERWEAVE;`}</Code>
      <Code title="Drop a role">{`UNWEAVE ROLE analyst;`}</Code>
      <Code title="List roles">{`SHOW ROLES;`}</Code>

      <H3>Permissions</H3>
      <Code>{`-- Grant operations on a bundle to a role
GRANT SECTION, COVER ON sensors TO analyst;
GRANT ALL ON sensors TO admin;

-- Revoke
REVOKE RETRACT ON sensors FROM analyst;`}</Code>
      <P>Grantable operations: <strong style={{ color: "#D0D8E4" }}>SECTION</strong> (insert), <strong style={{ color: "#D0D8E4" }}>COVER</strong> (query), <strong style={{ color: "#D0D8E4" }}>REDEFINE</strong> (update), <strong style={{ color: "#D0D8E4" }}>RETRACT</strong> (delete), <strong style={{ color: "#D0D8E4" }}>ALL</strong>.</P>

      <H3>Row-Level Security Policies</H3>
      <Code>{`-- Only let analyst see their own region
POLICY region_filter ON sensors
  FOR COVER, SECTION
  RESTRICT TO (region = CURRENT_ROLE.region)
  TO analyst;

DROP POLICY region_filter ON sensors;
SHOW POLICIES ON sensors;`}</Code>

      <H3>Audit Logging</H3>
      <Code>{`-- Enable audit on specific operations
AUDIT sensors ON SECTION, REDEFINE, RETRACT;

-- View audit log
AUDIT SHOW sensors;
AUDIT SHOW sensors SINCE '2024-06-01';
AUDIT SHOW sensors ROLE analyst;

-- Disable
AUDIT sensors OFF;`}</Code>

      <H3>Session Commands</H3>
      <Code>{`SHOW CURRENT ROLE;
SHOW SESSION;`}</Code>
    </Section>
  );
}

function ConstraintsSection() {
  return (
    <Section id="constraints" title="Constraints">
      <P>GQL supports CHECK constraints, UNIQUE constraints, and foreign-key-like MORPHISM constraints via the GAUGE CONSTRAIN syntax.</P>

      <H3>GAUGE CONSTRAIN</H3>
      <Code title="Syntax">{`GAUGE <bundle> CONSTRAIN (
  ADD CHECK (<expression>) AS <name>,
  ADD UNIQUE (<field> [, ...]) AS <name>,
  ADD MORPHISM <field> -> <target_bundle>(<target_field>)
);`}</Code>

      <Code title="Examples">{`GAUGE orders CONSTRAIN (
  ADD CHECK (total > 0) AS positive_total,
  ADD CHECK (quantity >= 1) AS min_quantity,
  ADD UNIQUE (customer_id, order_date) AS unique_daily_order,
  ADD MORPHISM customer_id -> customers(id)
);`}</Code>

      <H3>Remove Constraint</H3>
      <Code>{`GAUGE orders UNCONSTRAIN positive_total;`}</Code>

      <H3>View Constraints</H3>
      <Code>{`SHOW CONSTRAINTS ON orders;
SHOW MORPHISMS ON orders;`}</Code>

      <Table
        headers={["Constraint Type", "SQL Equivalent", "Description"]}
        rows={[
          ["CHECK", "CHECK", "Arbitrary boolean expression on field values"],
          ["UNIQUE", "UNIQUE", "No duplicate values for the field combination"],
          ["MORPHISM", "FOREIGN KEY REFERENCES", "Referential integrity — target record must exist"],
        ]}
      />
    </Section>
  );
}

function IndexesSection() {
  return (
    <Section id="indexes" title="Indexes">
      <P>Indexes in GIGI are bitmap indexes on fiber fields. They enable O(|bucket|) lookups via COVER ON — only scanning records that match the indexed value.</P>

      <H3>Creating Indexes</H3>
      <Code title="At bundle creation">{`BUNDLE sensors
  BASE (id NUMERIC)
  FIBER (
    city   CATEGORICAL INDEX,
    region CATEGORICAL INDEX,
    temp   NUMERIC
  );`}</Code>
      <Code title="Add index later">{`-- Via REST API
POST /v1/bundles/sensors/add-index
{ "field": "status" }`}</Code>

      <H3>Index Types</H3>
      <Table
        headers={["Type", "Structure", "Best For", "Complexity"]}
        rows={[
          ["Bitmap Index", "Value → HashSet of keys", "CATEGORICAL fields with moderate cardinality", "O(|bucket|) lookup"],
          ["Base Hash", "Key → Record (built-in)", "Point queries on key fields", "O(1) always"],
        ]}
      />

      <H3>When to Use ON vs WHERE</H3>
      <Table
        headers={["Clause", "Requires Index", "Complexity", "Use When"]}
        rows={[
          ["COVER ... ON", "Yes (INDEX modifier)", "O(|bucket|)", "Field has an index — fast bitmap lookup"],
          ["COVER ... WHERE", "No", "O(n) scan", "No index, or complex predicate"],
          ["COVER ... ON ... WHERE", "ON field indexed", "O(|bucket|) then filter", "Index narrows, WHERE refines"],
        ]}
      />

      <H3>View Indexes</H3>
      <Code>{`SHOW INDEXES ON sensors;`}</Code>

      <H3>Rebuild Index</H3>
      <Code>{`REBUILD INDEX sensors;
REBUILD INDEX sensors ON city;`}</Code>
    </Section>
  );
}

function PreparedSection() {
  return (
    <Section id="prepared" title="Prepared Statements">
      <P>GQL supports parameterized prepared statements for repeated queries with variable inputs — preventing injection and improving query plan caching.</P>

      <H3>PREPARE</H3>
      <Code>{`PREPARE point_lookup AS SECTION sensors AT id = $1;
PREPARE range_query AS COVER sensors ON city = $1 WHERE temp > $2;`}</Code>

      <H3>EXECUTE</H3>
      <Code>{`EXECUTE point_lookup (42);
EXECUTE range_query ('Moscow', -25);`}</Code>

      <H3>DEALLOCATE</H3>
      <Code>{`DEALLOCATE point_lookup;
DEALLOCATE ALL;`}</Code>

      <H3>View Prepared Statements</H3>
      <Code>{`SHOW PREPARED;`}</Code>
    </Section>
  );
}

function TriggersSection() {
  return (
    <Section id="triggers" title="Triggers">
      <P>Triggers execute actions automatically when data changes — before or after SECTION/REDEFINE/RETRACT operations.</P>

      <H3>ON (Post-condition Trigger)</H3>
      <Code>{`-- Alert when temperature drops below threshold
ON SECTION sensors WHERE temp < -30 EXECUTE ALERT 'extreme_cold';`}</Code>

      <H3>BEFORE (Pre-check)</H3>
      <Code>{`-- Enforce referential integrity before insert
BEFORE SECTION orders
  CHECK (EXISTS SECTION customers AT id = NEW.customer_id);`}</Code>

      <H3>AFTER (Post-action)</H3>
      <Code>{`-- Log every update to an audit table
AFTER REDEFINE sensors EXECUTE (
  SECTION audit_log (
    bundle: 'sensors',
    action: 'redefine',
    timestamp: NOW()
  )
);`}</Code>

      <H3>Manage Triggers</H3>
      <Code>{`DROP TRIGGER extreme_cold ON sensors;
SHOW TRIGGERS ON sensors;`}</Code>
    </Section>
  );
}

function MaintenanceSection() {
  return (
    <Section id="maintenance" title="Maintenance">
      <P>Database maintenance commands for optimizing storage, refreshing statistics, and verifying integrity.</P>

      <H3>COMPACT</H3>
      <P>Rewrites the WAL from the current engine state — removes old entries and reclaims space.</P>
      <Code>{`COMPACT sensors;
COMPACT sensors ANALYZE;  -- compact + refresh statistics`}</Code>

      <H3>ANALYZE</H3>
      <P>Refresh internal statistics (cardinality, distribution) used by the query planner.</P>
      <Code>{`ANALYZE sensors;
ANALYZE sensors ON temp;
ANALYZE sensors FULL;`}</Code>

      <H3>VACUUM</H3>
      <P>Reclaim disk space from deleted records.</P>
      <Code>{`VACUUM sensors;
VACUUM sensors FULL;  -- complete rewrite`}</Code>

      <H3>REBUILD INDEX</H3>
      <Code>{`REBUILD INDEX sensors;
REBUILD INDEX sensors ON city;`}</Code>

      <H3>CHECK &amp; REPAIR</H3>
      <Code>{`CHECK sensors;   -- verify WAL/index/hash integrity
REPAIR sensors;  -- auto-fix detected issues`}</Code>

      <H3>STORAGE</H3>
      <Code>{`STORAGE sensors;  -- storage usage report`}</Code>

      <H3>Session Settings</H3>
      <Code>{`SET TOLERANCE 0.01;     -- curvature tolerance threshold
SET EMIT DHOOM;         -- output format
RESET TOLERANCE;
RESET ALL;
SHOW SETTINGS;
SHOW SESSION;`}</Code>
    </Section>
  );
}

function BackupSection() {
  return (
    <Section id="backup" title="Backup & Restore">
      <H3>BACKUP</H3>
      <Code>{`-- Single bundle backup
BACKUP sensors TO 'sensors.gigi';

-- All bundles
BACKUP ALL TO 'full_backup.gigi';

-- Compressed
BACKUP sensors TO 'sensors.gigi.zst' COMPRESS;

-- Incremental (only changes since date)
BACKUP sensors TO 'incr.gigi' INCREMENTAL SINCE '2024-06-01';`}</Code>

      <H3>RESTORE</H3>
      <Code>{`-- Standard restore
RESTORE sensors FROM 'sensors.gigi';

-- Restore to a specific snapshot
RESTORE sensors FROM 'sensors.gigi' AT SNAPSHOT 'pre_migration';

-- Restore with a new name
RESTORE sensors FROM 'sensors.gigi' AS sensors_restored;`}</Code>

      <H3>VERIFY</H3>
      <Code>{`VERIFY BACKUP 'sensors.gigi';
SHOW BACKUPS;`}</Code>
    </Section>
  );
}

function ImportExportSection() {
  return (
    <Section id="import" title="Import & Export">
      <H3>INGEST (Import)</H3>
      <Code>{`-- From file
INGEST sensors FROM 'data.csv' FORMAT CSV;
INGEST sensors FROM 'data.json' FORMAT JSON;
INGEST sensors FROM 'data.jsonl' FORMAT JSONL;
INGEST sensors FROM 'dump.sql' FORMAT SQL;

-- From stdin (streaming)
INGEST sensors FROM STDIN FORMAT JSONL;`}</Code>
      <Table
        headers={["Format", "Description"]}
        rows={[
          ["CSV", "Comma-separated values with header row"],
          ["JSON", "Array of objects"],
          ["JSONL", "Newline-delimited JSON (one object per line)"],
          ["DHOOM", "GIGI's native binary format (most compact)"],
          ["SQL", "SQL INSERT statements"],
        ]}
      />

      <H3>NDJSON Stream Ingest (REST)</H3>
      <Code title="POST /v1/bundles/sensors/stream">{`Content-Type: application/x-ndjson

{"id":1,"city":"Moscow","temp":-31.9}
{"id":2,"city":"Tokyo","temp":22.1}
{"id":3,"city":"Nairobi","temp":28.4}`}</Code>
      <P>Maximum stream size: 256 MB per request.</P>

      <H3>TRANSPLANT (Move Data)</H3>
      <Code>{`-- Move old data to archive
TRANSPLANT sensors INTO sensors_archive
  WHERE date < 20240101
  RETRACT SOURCE;  -- delete from source after copy`}</Code>

      <H3>GENERATE BASE</H3>
      <Code>{`-- Generate sequential base points
GENERATE BASE timeseries
  FROM date=20240101 TO date=20241231 STEP 1;`}</Code>

      <H3>FILL (Data Repair)</H3>
      <Code>{`-- Fill missing values
FILL sensors ON temp USING INTERPOLATE LINEAR;
FILL sensors ON temp USING TRANSPORT;  -- parallel transport fill`}</Code>

      <H3>Export (REST)</H3>
      <Code>{`GET /v1/bundles/sensors/export
-- Returns: JSON array of all records

POST /v1/bundles/sensors/import
-- Body: JSON array of records to import`}</Code>
    </Section>
  );
}

function EncryptionSection() {
  return (
    <Section id="encryption" title="Geometric Encryption">
      <P>GIGI implements geometric encryption via gauge transforms — affine transformations on fiber values that preserve curvature invariance. This means you can run analytics (curvature, spectral gap, anomaly detection) directly on encrypted data without decryption.</P>

      <H3>How It Works</H3>
      <P>Each fiber field gets an affine transform: <strong style={{ color: "#D0D8E4" }}>v_enc = a · v + b</strong> (encrypt), <strong style={{ color: "#D0D8E4" }}>v = (v_enc - b) / a</strong> (decrypt).</P>
      <Table
        headers={["Property", "Value"]}
        rows={[
          ["Key structure", "Per-bundle GaugeKey with per-field FieldTransform (scale a, offset b)"],
          ["Key derivation", "32-byte seed + field names → deterministic KDF"],
          ["Scale range", "a ∈ [0.1, 10.0), a ≠ 0"],
          ["Offset range", "b ∈ [-1000, 1000)"],
          ["Applies to", "NUMERIC and TIMESTAMP fields only"],
          ["Skips", "CATEGORICAL, BOOLEAN, TEXT (pass through unchanged)"],
        ]}
      />

      <H3>Curvature Invariance</H3>
      <P>The key insight: scalar curvature K = Var/range² is invariant under affine transforms because both variance and range scale by a²:</P>
      <Code>{`K = Var(a·v+b) / (a·range)² = a²·Var(v) / a²·range² = Var(v) / range² = K`}</Code>
      <P>This means curvature, confidence, capacity, spectral gap, and anomaly detection all produce the same results on encrypted data as on plain data.</P>

      <H3>Usage</H3>
      <Code title="Create an encrypted bundle">{`BUNDLE secrets
  BASE (id NUMERIC)
  FIBER (
    ssn    NUMERIC,
    salary NUMERIC RANGE 500000,
    rating NUMERIC RANGE 5
  ) ENCRYPTED;`}</Code>
      <Note type="info">Only point queries that return human-readable results require decryption. All geometric analytics run directly on the encrypted representation.</Note>
    </Section>
  );
}

function RESTSection() {
  return (
    <Section id="rest" title="REST API">
      <P>GIGI Stream exposes a comprehensive REST API on the configured port (default: 3142). All endpoints accept and return JSON. Authentication is via the X-API-Key header when GIGI_API_KEY is set.</P>

      <H3>Authentication</H3>
      <Code>{`# Every request must include the API key header:
curl -H "X-API-Key: YOUR_KEY" https://your-server/v1/health`}</Code>

      <H3>System Endpoints</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["GET", "/v1/health", "Health check — uptime, bundle count, record count"],
          ["GET", "/v1/openapi.json", "OpenAPI 3.0 specification"],
        ]}
      />

      <H3>Bundle Management</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["GET", "/v1/bundles", "List all bundles"],
          ["POST", "/v1/bundles", "Create bundle (schema in body)"],
          ["DELETE", "/v1/bundles/{name}", "Drop bundle"],
          ["GET", "/v1/bundles/{name}/schema", "Get bundle schema"],
          ["GET", "/v1/bundles/{name}/stats", "Bundle statistics"],
        ]}
      />
      <Code title="POST /v1/bundles — Create">{`{
  "name": "sensors",
  "schema": {
    "fields": {
      "id": "numeric",
      "city": "categorical",
      "temp": "numeric",
      "status": "categorical"
    },
    "keys": ["id"],
    "indexes": ["city"],
    "ranges": { "temp": 80 },
    "defaults": { "status": "normal" }
  }
}`}</Code>

      <H3>Write Operations</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["POST", "/v1/bundles/{name}/insert", "Insert records"],
          ["POST", "/v1/bundles/{name}/points", "Insert (alias)"],
          ["POST", "/v1/bundles/{name}/stream", "NDJSON stream ingest (max 256MB)"],
          ["POST", "/v1/bundles/{name}/upsert", "Insert or update existing"],
          ["POST", "/v1/bundles/{name}/update", "Update by key (supports RETURNING)"],
          ["POST", "/v1/bundles/{name}/delete", "Delete by key (supports RETURNING)"],
          ["POST", "/v1/bundles/{name}/increment", "Atomic increment/decrement"],
          ["POST", "/v1/bundles/{name}/truncate", "Delete all records"],
        ]}
      />
      <Code title="Insert records">{`POST /v1/bundles/sensors/insert
{
  "records": [
    { "id": 1, "city": "Moscow", "temp": -31.9, "status": "cold" },
    { "id": 2, "city": "Tokyo", "temp": 22.1, "status": "normal" }
  ]
}`}</Code>
      <Code title="Response">{`{
  "status": "inserted",
  "count": 2,
  "total": 502,
  "curvature": 0.0023,
  "confidence": 0.9977
}`}</Code>

      <H3>Read Operations</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["GET", "/v1/bundles/{name}/get?key=val", "Point query O(1)"],
          ["GET", "/v1/bundles/{name}/points/{field}/{value}", "Get by field/value"],
          ["GET", "/v1/bundles/{name}/range?field=val", "Range query"],
          ["GET", "/v1/bundles/{name}/points[?limit=&offset=]", "List all records (paginated)"],
          ["POST", "/v1/bundles/{name}/query", "Filtered query (conditions, sort, limit, search)"],
          ["POST", "/v1/bundles/{name}/count", "Count matching records"],
          ["POST", "/v1/bundles/{name}/exists", "Existence check"],
          ["GET", "/v1/bundles/{name}/distinct/{field}", "Distinct values for a field"],
        ]}
      />
      <Code title="Filtered query">{`POST /v1/bundles/sensors/query
{
  "conditions": [
    { "field": "city", "op": "eq", "value": "Moscow" },
    { "field": "temp", "op": "lt", "value": 0 }
  ],
  "sort": [{ "field": "temp", "order": "asc" }],
  "limit": 10,
  "offset": 0,
  "fields": ["city", "temp", "status"],
  "search": "cold"
}`}</Code>

      <H3>Update & Delete by Field Path</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["PATCH", "/v1/bundles/{name}/points/{field}/{value}", "Update by field/value"],
          ["DELETE", "/v1/bundles/{name}/points/{field}/{value}", "Delete by field/value"],
          ["PATCH", "/v1/bundles/{name}/points", "Bulk update by filter"],
          ["POST", "/v1/bundles/{name}/bulk-delete", "Bulk delete by filter"],
        ]}
      />

      <H3>Analytics</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["GET", "/v1/bundles/{name}/curvature", "Curvature report (K, confidence, capacity)"],
          ["GET", "/v1/bundles/{name}/spectral", "Spectral analysis (λ₁, diameter)"],
          ["GET", "/v1/bundles/{name}/consistency", "Čech H¹ consistency check"],
          ["POST", "/v1/bundles/{name}/join", "Pullback join"],
          ["POST", "/v1/bundles/{name}/aggregate", "GROUP BY aggregation"],
        ]}
      />

      <H3>Schema Evolution</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["POST", "/v1/bundles/{name}/add-field", "Add fiber field"],
          ["POST", "/v1/bundles/{name}/add-index", "Add field index"],
        ]}
      />

      <H3>Transactions & Plans</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["POST", "/v1/bundles/{name}/transaction", "Atomic transaction"],
          ["POST", "/v1/bundles/{name}/explain", "Query execution plan"],
        ]}
      />

      <H3>Export / Import</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["GET", "/v1/bundles/{name}/export", "Export as JSON"],
          ["POST", "/v1/bundles/{name}/import", "Import from JSON"],
        ]}
      />

      <H3>GQL Endpoint</H3>
      <Code>{`POST /v1/gql
{ "query": "SHOW BUNDLES;" }`}</Code>
      <P>Execute any GQL statement via REST. Returns the query result as JSON.</P>

      <H3>Error Responses</H3>
      <Table
        headers={["Status", "Meaning"]}
        rows={[
          ["200", "OK — request succeeded"],
          ["201", "Created — bundle or record created"],
          ["400", "Bad Request — invalid query, schema, or parameters"],
          ["401", "Unauthorized — missing or invalid API key"],
          ["404", "Not Found — bundle or record doesn't exist"],
          ["409", "Conflict — optimistic concurrency version mismatch"],
          ["429", "Rate Limited — exceeded request quota"],
        ]}
      />
      <Code title="Error format">{`{ "error": "Bundle 'foo' not found" }`}</Code>
    </Section>
  );
}

function WebSocketSection() {
  return (
    <Section id="websocket" title="WebSocket Protocol">
      <P>GIGI Stream provides a WebSocket endpoint for real-time operations — inserts, queries, subscriptions, and curvature monitoring.</P>

      <H3>Connection</H3>
      <Code>{`ws://your-server:3142/ws`}</Code>

      <H3>Commands</H3>
      <Table
        headers={["Command", "Syntax", "Response"]}
        rows={[
          ["INSERT", 'INSERT bundle_name\\n<DHOOM / JSON data>', 'OK inserted=N total=N K=0.0023 confidence=0.9977'],
          ["QUERY", 'QUERY bundle WHERE field = "value"', 'RESULT {...}\\nMETA confidence=X curvature=Y'],
          ["RANGE", 'RANGE bundle WHERE field = "value"', 'RESULT [...]\\nMETA count=N confidence=X curvature=Y'],
          ["SUBSCRIBE", 'SUBSCRIBE bundle WHERE field = "value"', 'SUBSCRIBED ... + push updates'],
          ["CURVATURE", "CURVATURE bundle[.field]", "CURVATURE K=X confidence=Y capacity=Z"],
          ["CONSISTENCY", "CONSISTENCY bundle", "CONSISTENCY h1=0 cocycles=0"],
        ]}
      />

      <H3>Subscriptions</H3>
      <P>After subscribing, the server pushes matching records as they are inserted:</P>
      <Code>{`-- Subscribe to Moscow sensor readings
SUBSCRIBE sensors WHERE city = "Moscow"

-- Server response:
SUBSCRIBED sensors WHERE city = "Moscow"

-- On matching insert, server pushes:
UPDATE sensors {"id":42,"city":"Moscow","temp":-31.9,"status":"cold"}`}</Code>

      <H3>DHOOM Format</H3>
      <P>GIGI's native binary serialization format, optimized for data where most fields are derivable from structure. Achieves 66-84% fewer tokens than JSON for LLM context injection.</P>
    </Section>
  );
}

function SDKSection() {
  return (
    <Section id="sdk" title="JavaScript SDK">
      <P>The GIGI JavaScript SDK provides a typed client for Node.js and browser environments. Available at <code style={{ fontSize: 12, background: "#0E1020", padding: "2px 6px", borderRadius: 3, fontFamily: MONO }}>sdk/js/</code> in the GIGI repository.</P>

      <H3>Installation & Setup</H3>
      <Code title="Initialize">{`import { GIGIClient } from 'gigi-sdk';

const db = new GIGIClient('https://gigi-stream.fly.dev', {
  apiKey: 'YOUR_API_KEY'
});`}</Code>

      <H3>Client Methods</H3>
      <Table
        headers={["Method", "Returns", "Description"]}
        rows={[
          ["db.health()", "HealthResult", "Server health check"],
          ["db.listBundles()", "string[]", "List all bundle names"],
          ["db.bundle(name)", "BundleHandle", "Get a bundle handle for operations"],
          ["db.gql(query)", "any", "Execute raw GQL query"],
          ["db.openapi()", "object", "Fetch OpenAPI spec"],
          ["db.connect()", "void", "Open WebSocket connection"],
          ["db.close()", "void", "Close WebSocket"],
        ]}
      />

      <H3>Bundle Handle Methods</H3>
      <H4>Schema Operations</H4>
      <Table
        headers={["Method", "Returns", "Description"]}
        rows={[
          ["b.create(schema)", "void", "Create the bundle with schema"],
          ["b.drop()", "void", "Drop the bundle"],
          ["b.schema()", "BundleSchema", "Get the bundle schema"],
          ["b.addField(name, type, default?)", "void", "Add a fiber field"],
          ["b.addIndex(field)", "void", "Add a bitmap index"],
        ]}
      />

      <H4>Write Operations</H4>
      <Table
        headers={["Method", "Returns", "Description"]}
        rows={[
          ["b.insert(records)", "InsertResult", "Insert one or more records"],
          ["b.upsert(record)", "UpsertResult", "Insert or update"],
          ["b.update(key, fields, opts?)", "UpdateResult", "Update by key"],
          ["b.deleteRecord(key, opts?)", "DeleteResult", "Delete by key"],
          ["b.increment(key, field, amount)", "IncrementResult", "Atomic increment"],
          ["b.bulkUpdate(opts)", "BulkUpdateResult", "Update multiple by filter"],
          ["b.bulkDelete(conditions)", "DeleteResult", "Delete multiple by filter"],
          ["b.truncate()", "void", "Delete all records"],
          ["b.transaction(ops)", "TransactionResult", "Atomic multi-op transaction"],
        ]}
      />

      <H4>Read Operations</H4>
      <Table
        headers={["Method", "Returns", "Description"]}
        rows={[
          ["b.get(key)", "Record | null", "O(1) point query"],
          ["b.getByField(field, value)", "Record[]", "Get by field path"],
          ["b.query(opts)", "QueryResult", "Filtered query with sort/limit"],
          ["b.listAll(opts?)", "Record[]", "List all records (paginated)"],
          ["b.range(field?, value?)", "RangeResult", "Range query"],
          ["b.count(conditions?)", "number", "Count matching records"],
          ["b.exists(conditions)", "boolean", "Existence check"],
          ["b.distinct(field)", "any[]", "Unique values for a field"],
        ]}
      />

      <H4>Analytics</H4>
      <Table
        headers={["Method", "Returns", "Description"]}
        rows={[
          ["b.curvature()", "CurvatureReport", "Scalar curvature, confidence, capacity"],
          ["b.spectral()", "SpectralReport", "Spectral gap λ₁, diameter"],
          ["b.checkConsistency()", "ConsistencyReport", "Čech H¹ cohomology check"],
          ["b.aggregate(opts)", "AggregateResult", "GROUP BY aggregation"],
          ["b.join(right, leftField, rightField)", "JoinResult", "Pullback join"],
          ["b.explain(opts)", "QueryPlan", "Query execution plan"],
          ["b.stats()", "BundleStats", "Bundle statistics"],
        ]}
      />

      <H4>Real-time</H4>
      <Code>{`// Subscribe to real-time updates
const sub = db.bundle('sensors')
  .where('city', 'Moscow')
  .subscribe((record) => {
    console.log('New Moscow reading:', record);
  });`}</Code>

      <H3>Edge Client</H3>
      <Code>{`import { GIGIEdge } from 'gigi-sdk';

const edge = new GIGIEdge('https://gigi-stream.fly.dev');

// All reads are local O(1)
const record = edge.bundle('sensors').get({ id: 42 });

// Writes queue locally, sync when connected
edge.bundle('sensors').insert([{ id: 100, city: 'Berlin', temp: 8.5 }]);

// Sync with remote (sheaf gluing)
const report = await edge.sync();
// report.conflicts = [] when h1 = 0`}</Code>
    </Section>
  );
}

function EdgeSection() {
  return (
    <Section id="edge" title="Edge Sync">
      <P>GIGI Edge provides local-first operation with synchronization to a remote GIGI Stream server. All reads are local O(1). Writes queue locally and sync when connected, using sheaf gluing for conflict detection.</P>

      <H3>Architecture</H3>
      <Table
        headers={["Component", "Description"]}
        rows={[
          ["Local Engine", "Full GIGI engine running on the edge device"],
          ["Sync Queue", "Ordered list of local operations waiting to be pushed"],
          ["Remote URL", "The GIGI Stream server to sync with"],
        ]}
      />

      <H3>Sync Protocol (Sheaf Gluing)</H3>
      <Code title="Sync steps">{`1. Push local operations to server (REST API)
2. Server applies operations, returns H¹ (conflict check)
3. Pull new records from server
4. Apply pulled records locally
5. H¹ = 0 → clean merge
   H¹ > 0 → conflicts returned with local/remote values`}</Code>

      <H3>Conflict Detection</H3>
      <P>Conflicts are detected via Čech cohomology — when the same key has different values on local and remote:</P>
      <Code>{`// Conflict structure
{
  "bundle": "sensors",
  "field": "temp",
  "key": [["id", 42]],
  "local_value": -31.9,
  "remote_value": -28.5
}`}</Code>

      <H3>REST API</H3>
      <Table
        headers={["Method", "Path", "Description"]}
        rows={[
          ["POST", "/v1/sync", "Trigger sync (push local ops, pull remote changes)"],
          ["GET", "/v1/status", "Edge status (connected, queue depth, last sync time)"],
        ]}
      />
    </Section>
  );
}

function ConfigSection() {
  return (
    <Section id="config" title="Configuration">
      <H3>Environment Variables</H3>
      <Table
        headers={["Variable", "Default", "Description"]}
        rows={[
          ["PORT", "3142", "Server listen port"],
          ["GIGI_DATA_DIR", "./gigi_data", "Directory for WAL and data files"],
          ["GIGI_API_KEY", "(none)", "API key for authentication. When set, all requests must include X-API-Key header"],
          ["GIGI_RATE_LIMIT", "0 (unlimited)", "Maximum requests per window per IP address"],
          ["GIGI_RATE_WINDOW", "60", "Rate limit window duration in seconds"],
        ]}
      />

      <H3>WAL (Write-Ahead Log)</H3>
      <Table
        headers={["Setting", "Value", "Description"]}
        rows={[
          ["Checkpoint interval", "10,000 ops", "Auto-checkpoint after this many operations"],
          ["Format", "[4B length][1B op][payload][4B CRC32]", "Binary WAL entry format"],
          ["Crash recovery", "Automatic", "WAL replayed on startup"],
          ["Compaction", "COMPACT statement", "Rewrite WAL from current state"],
        ]}
      />
      <Table
        headers={["Op Code", "Hex", "Operation"]}
        rows={[
          ["INSERT", "0x01", "Insert a record"],
          ["CREATE_BUNDLE", "0x02", "Create a bundle schema"],
          ["UPDATE", "0x03", "Partial field update"],
          ["DELETE", "0x04", "Delete a record"],
          ["DROP_BUNDLE", "0x05", "Drop a bundle"],
          ["CHECKPOINT", "0xFF", "Marker: all prior entries are flushed"],
        ]}
      />

      <H3>Limits</H3>
      <Table
        headers={["Limit", "Value"]}
        rows={[
          ["NDJSON stream max size", "256 MB per request"],
          ["Hash space", "64-bit (ℤ₂₆₄)"],
          ["Birthday bound", "~4.3 × 10⁹ records (secondary collision map above this)"],
          ["Spectral iterations", "300 (power iteration for eigenvalue)"],
          ["WebSocket broadcast buffer", "1,024 messages"],
        ]}
      />
    </Section>
  );
}

function FunctionsSection() {
  return (
    <Section id="functions" title="Functions Reference">
      <H3>Aggregate Functions</H3>
      <Table
        headers={["Function", "Syntax", "Description", "Example"]}
        rows={[
          ["COUNT", "count(*) or count(field)", "Count records or non-null values", "INTEGRATE s MEASURE count(*)"],
          ["SUM", "sum(field)", "Sum of numeric values", "INTEGRATE s OVER city MEASURE sum(sales)"],
          ["AVG", "avg(field)", "Arithmetic mean", "INTEGRATE s MEASURE avg(temp)"],
          ["MIN", "min(field)", "Minimum value", "INTEGRATE s MEASURE min(temp)"],
          ["MAX", "max(field)", "Maximum value", "INTEGRATE s MEASURE max(temp)"],
        ]}
      />

      <H3>Math Functions (Spec)</H3>
      <Table
        headers={["Function", "Description"]}
        rows={[
          ["ABS(x)", "Absolute value"],
          ["ROUND(x [,n])", "Round to n decimal places"],
          ["CEIL(x)", "Smallest integer ≥ x"],
          ["FLOOR(x)", "Largest integer ≤ x"],
          ["TRUNC(x [,n])", "Truncate to n decimal places"],
          ["POWER(x, y)", "x raised to y"],
          ["SQRT(x)", "Square root"],
          ["CBRT(x)", "Cube root"],
          ["LOG(x)", "Natural logarithm"],
          ["LOG10(x)", "Base-10 logarithm"],
          ["LOG2(x)", "Base-2 logarithm"],
          ["EXP(x)", "e raised to x"],
          ["MOD(x, y)", "Remainder of x / y"],
          ["SIGN(x)", "-1, 0, or 1"],
          ["GREATEST(a, b, ...)", "Maximum of arguments"],
          ["LEAST(a, b, ...)", "Minimum of arguments"],
          ["PI()", "π = 3.14159..."],
          ["RANDOM()", "Random float in [0, 1)"],
        ]}
      />

      <H3>String Functions (Spec)</H3>
      <Table
        headers={["Function", "Description"]}
        rows={[
          ["UPPER(s)", "Convert to uppercase"],
          ["LOWER(s)", "Convert to lowercase"],
          ["INITCAP(s)", "Capitalize first letter of each word"],
          ["LENGTH(s)", "String length in characters"],
          ["SUBSTR(s, start [,len])", "Substring extraction"],
          ["LEFT(s, n)", "First n characters"],
          ["RIGHT(s, n)", "Last n characters"],
          ["CONCAT(a, b, ...)", "Concatenate strings"],
          ["CONCAT_WS(sep, a, b, ...)", "Concatenate with separator"],
          ["REPLACE(s, from, to)", "Replace all occurrences"],
          ["TRIM(s)", "Remove leading/trailing whitespace"],
          ["LTRIM(s) / RTRIM(s)", "Left/right trim"],
          ["LPAD(s, n, pad) / RPAD(s, n, pad)", "Left/right pad to length n"],
          ["REVERSE(s)", "Reverse string"],
          ["REPEAT(s, n)", "Repeat string n times"],
          ["POSITION(sub IN s)", "Index of substring (1-based)"],
          ["SPLIT(s, delim)", "Split into array"],
          ["MD5(s)", "MD5 hash as hex string"],
        ]}
      />

      <H3>Date/Time Functions (Spec)</H3>
      <Table
        headers={["Function", "Description"]}
        rows={[
          ["NOW()", "Current timestamp (epoch ms)"],
          ["TODAY()", "Current date as YYYYMMDD integer"],
          ["EPOCH(ts)", "Convert timestamp to epoch ms"],
          ["FROM_EPOCH(ms)", "Convert epoch ms to timestamp"],
          ["DATEPART(part, ts)", "Extract year/month/day/hour/minute/second"],
          ["DATETRUNC(part, ts)", "Truncate to year/month/day boundary"],
          ["DATEADD(part, n, ts)", "Add n units to timestamp"],
          ["DATEDIFF(part, ts1, ts2)", "Difference in units between timestamps"],
          ["FORMAT_DATE(ts, fmt)", "Format timestamp as string"],
          ["PARSE_DATE(s, fmt)", "Parse string to timestamp"],
        ]}
      />

      <H3>Conditional Expressions (Spec)</H3>
      <Code>{`-- CLASSIFY (CASE equivalent)
CLASSIFY
  WHEN temp < 0 THEN 'freezing'
  WHEN temp < 20 THEN 'cool'
  ELSE 'warm'
AS temp_category

-- IF function
IF(temp > 30, 'hot', 'ok')

-- RESOLVE (COALESCE equivalent)
RESOLVE(nickname, username, 'anonymous')

-- VOIDIF (NULLIF equivalent)
VOIDIF(score, 0)`}</Code>

      <H3>Geometric State Functions</H3>
      <Table
        headers={["Function", "Returns", "Description"]}
        rows={[
          ["CONFIDENCE()", "Float 0–1", "Current bundle confidence = 1/(1+K)"],
          ["CURVATURE()", "Float", "Current scalar curvature K"],
          ["ANOMALY()", "Boolean", "Whether the last write was anomalous"],
          ["Z_SCORE()", "Float", "Z-score of last inserted value vs. distribution"],
          ["CAPACITY()", "Integer", "Remaining safe capacity C = τ/K"],
          ["CURRENT_ROLE()", "String", "Active session role name"],
          ["CURRENT_BUNDLE()", "String", "Active bundle context"],
          ["VERSION()", "String", "GIGI engine version"],
        ]}
      />
    </Section>
  );
}

function ReservedWordsSection() {
  const words = [
    "ADD", "AGAINST", "ALL", "ALONG", "ANOMALIES", "ARITHMETIC", "AS", "ASC", "AT", "ATLAS", "AUTO",
    "BARE", "BASE", "BEGIN", "BETTI", "BOOLEAN", "BOTTLENECK", "BUNDLE", "BY",
    "CALIBRATE", "CAPACITY", "CATEGORICAL", "CHARACTERISTIC", "CHECK", "CLASSIFY", "CLUSTER",
    "COBOUNDARY", "COCYCLE", "COARSEN", "COLLAPSE", "COMMIT", "COMPLETENESS", "CONDUCTANCE",
    "CONFIDENCE", "CONSISTENCY", "CORRELATE", "COVER", "CRITICAL", "CSV", "CURVATURE", "CURVED",
    "DEFAULT", "DEFINED", "DESC", "DESCRIBE", "DEVIATION", "DIFF", "DHOOM", "DISTINCT", "DIVERGENCE",
    "DRIFT", "DROP", "DOUBLECOVER",
    "ELSE", "EMIT", "ENTROPY", "EULER", "EXISTS", "EXPLAIN",
    "FIBER", "FIRST", "FISHER", "FLAT", "FLOW", "FREEENERGY", "FROM", "FULL",
    "GAUGE", "GEODESIC", "GLUE",
    "HAVING", "HEALTH", "HOLONOMY",
    "IN", "INDEX", "INTEGRATE", "INTERSECT", "INTO", "ISOLATION",
    "JSON",
    "LAPLACIAN", "LENS", "LEVELS",
    "MATCHES", "MATERIALIZE", "MEASURE", "MINUS", "MIXING", "MODE", "MUTUAL",
    "NEAR", "NULLABLE", "NUMERIC",
    "ON", "ONTO", "OPTIONS", "OR", "OUTLIER", "OVER", "OVERLAP",
    "PARTITION", "PERCENTILE", "PERCENTRANK", "PHASE", "PREDICT", "PRESERVE",
    "PRODUCT", "PROFILE", "PROJECT",
    "RANGE", "RANK", "RECALL", "REDEFINE", "REFRESH", "REPAIR", "REQUIRED", "RESOLVE",
    "RESTRICT", "RETRACT", "RICCI", "ROLLBACK", "ROWS",
    "SAVEPOINT", "SCALAR", "SECTION", "SECTIONS", "SECTIONAL", "SEGMENT", "SET", "SHIFT",
    "SHOW", "SIGMA", "SIMILAR", "SKIP", "SNAPSHOT", "SPECTRAL", "SQL", "SUBSCRIBE", "SUBTRACT",
    "TEMPERATURE", "TEST", "TEXT", "THEN", "THRESHOLD", "TIMESTAMP", "TO", "TOLERANCE", "TOP",
    "TRAIN", "TRANSLATE", "TRANSPORT", "TREND", "TRIVIALIZE",
    "UNION", "UNIQUE", "UPSERT",
    "VACUUM", "VERIFY", "VOID",
    "WEAVE", "WHERE", "WILSON",
  ];
  return (
    <Section id="reserved" title="Reserved Words">
      <P>The following {words.length} identifiers are reserved by GQL and cannot be used as unquoted bundle or field names:</P>
      <div style={{ margin: "12px 0", padding: 16, background: "#080A12", borderRadius: 8, border: `1px solid ${BORDER}`, display: "flex", flexWrap: "wrap", gap: "4px 8px" }}>
        {words.map(w => (
          <span key={w} style={{ fontSize: 11, fontFamily: MONO, color: "#8090A0", padding: "2px 6px", background: "#0C0E18", borderRadius: 3 }}>{w}</span>
        ))}
      </div>
    </Section>
  );
}

function MathSection() {
  return (
    <Section id="math" title="Mathematical Reference">
      <P>GIGI is built on rigorous mathematics from differential geometry, algebraic topology, and information theory. This section provides the formal definitions.</P>

      <H3>Scalar Curvature</H3>
      <P>For a fiber field F with domain range R:</P>
      <Code>{`K = Var(F) / R²

where:
  Var(F) = (1/n) Σᵢ (vᵢ - μ)²    (population variance)
  R = max(F) - min(F)              (or declared RANGE)
  μ = (1/n) Σᵢ vᵢ                 (mean)`}</Code>

      <H3>Confidence</H3>
      <Code>{`confidence = 1 / (1 + K)

K = 0    → confidence = 1.0  (perfectly flat, fully predictable)
K = 0.01 → confidence ≈ 0.99
K = 1.0  → confidence = 0.5  (high variance)`}</Code>

      <H3>Davis Capacity</H3>
      <Code>{`C = τ / K

where τ is a configurable tolerance threshold (SET TOLERANCE).
C estimates how many more records can be inserted before
curvature exceeds the threshold — i.e., before confidence degrades.`}</Code>

      <H3>Partition Function</H3>
      <Code>{`Z(β, p) = Σ_q exp(-β · d(p, q))

where:
  β = inverse temperature (controls locality)
  d(p, q) = fiber metric distance between points p and q
  The sum runs over all records q in the bundle`}</Code>

      <H3>Spectral Gap</H3>
      <Code>{`λ₁ = smallest nonzero eigenvalue of L

where L = I - D^(-1/2) · W · D^(-1/2)  (normalized Laplacian)
  W = weighted adjacency matrix (from fiber metric)
  D = degree matrix

λ₁ > 0 → connected data (Cheeger's inequality bounds mixing)
λ₁ ≈ 0 → disconnected clusters`}</Code>

      <H3>Spectral Capacity</H3>
      <Code>{`C_sp = λ₁ · D²

where D = graph diameter
Large C_sp → well-connected with large spread = rich structure`}</Code>

      <H3>Čech Cohomology H¹</H3>
      <Code>{`H¹ = ker(δ₁) / im(δ₀)

where:
  δ₀: C⁰ → C¹  (coboundary map on 0-cochains)
  δ₁: C¹ → C²  (coboundary map on 1-cochains)

H¹ = 0 → globally consistent (sheaf gluing works)
H¹ > 0 → conflicts exist (obstructions to gluing)`}</Code>

      <H3>Holonomy</H3>
      <Code>{`Given a loop γ in the base space:
  H(γ) = Π_{edges in γ} transport(e)

H(γ) = identity → consistent around the loop
H(γ) ≠ identity → drift detected (the round-trip doesn't close)`}</Code>

      <H3>GIGI Hash</H3>
      <Code>{`1. Type-canonical encoding:
   Integer(v)  → [0x01] ++ v.to_le_bytes()
   Float(v)    → [0x02] ++ v.to_bits().to_le_bytes()
   Text(s)     → [0x03] ++ s.as_bytes()
   Bool(true)  → [0x04, 0x01]
   Bool(false) → [0x04, 0x00]
   Null        → [0x05]

2. Keyed mixing (wyhash-style):
   h = seed XOR (len * K1)
   for each 8-byte chunk: h = wyhash_mix(h XOR chunk)

3. Field composition:
   H(f1, f2, ..., fn) = h1 ⊕ rotate(h2, 17) ⊕ rotate(h3, 34) ⊕ ...

Properties:
   - Deterministic: same key → same hash, always
   - O(1) computation
   - Collision probability < 2⁻⁶⁴ per pair
   - Birthday bound: ~4.3 × 10⁹ (handled via secondary map)`}</Code>

      <H3>Fiber Product Metric</H3>
      <Code>{`g_F(v, w) = √( Σᵢ ωᵢ · gᵢ(vᵢ, wᵢ)² )

where:
  gᵢ = per-field metric (type-dependent, see Data Types section)
  ωᵢ = field weight (default 1.0)
  The sum runs over all fiber fields`}</Code>
    </Section>
  );
}

function ErrorsSection() {
  return (
    <Section id="errors" title="Error Reference">
      <Table
        headers={["Error", "Cause", "Fix"]}
        rows={[
          ["Bundle 'X' not found", "Bundle name doesn't exist", "Check SHOW BUNDLES; create the bundle first"],
          ["Bundle 'X' already exists", "Duplicate CREATE/BUNDLE", "Use a different name or COLLAPSE the existing one"],
          ["Key not found", "Point query for non-existent record", "Verify the key value exists"],
          ["Duplicate key", "Insert with existing key value", "Use UPSERT or change the key"],
          ["Field 'X' not in schema", "Reference to non-existent field", "Check DESCRIBE bundle; add the field"],
          ["Parse error at position N", "GQL syntax error", "Check syntax near the indicated position"],
          ["Unexpected token", "Malformed query", "Review the statement structure"],
          ["Missing required field 'X'", "REQUIRED field omitted on insert", "Provide the required field"],
          ["UNIQUE constraint violated", "Duplicate value on UNIQUE field", "Use a unique value"],
          ["Unauthorized", "Missing or invalid API key", "Set X-API-Key header"],
          ["Rate limited", "Too many requests", "Wait for the rate window to reset"],
          ["Version mismatch (409)", "Optimistic concurrency conflict", "Re-read the record and retry with current version"],
        ]}
      />
    </Section>
  );
}

function GlossarySection() {
  return (
    <Section id="glossary" title="Glossary">
      <Table
        headers={["Term", "Definition"]}
        rows={[
          ["Atlas", "A set of charts covering a manifold; in GQL, a transaction block (ATLAS BEGIN/COMMIT)"],
          ["Base Space (B)", "The key domain of a bundle, hashed to ℤ₂₆₄"],
          ["Bundle", "The fundamental data container — a fiber bundle (E, B, F, π) analogous to a table"],
          ["Capacity (C)", "C = τ/K — estimated safe remaining growth before quality degrades"],
          ["Čech Cohomology", "Algebraic topology tool for detecting inconsistencies (H¹ = 0 → consistent)"],
          ["Collapse", "Destroy a bundle and all its data (DROP TABLE equivalent)"],
          ["Confidence", "1/(1+K) — data quality metric ranging from 0 to 1"],
          ["Connection", "The geometric structure enabling parallel transport and curvature computation"],
          ["Cover", "An open cover query — returns all sections matching conditions"],
          ["Curvature (K)", "Var(fiber)/range² — measures data spread relative to domain"],
          ["DHOOM", "GIGI's native binary serialization format (66-84% more compact than JSON)"],
          ["Edge", "Local-first GIGI engine with sync capability to a remote Stream server"],
          ["Fiber (F)", "The data value space at each base point — product of all non-key fields"],
          ["Gauge Transform", "Affine transformation for geometric encryption (preserves curvature)"],
          ["GIGI Hash", "Wyhash-inspired 64-bit composite hash with type-canonical encoding"],
          ["Holonomy", "Parallel transport around a loop — nonzero holonomy signals drift"],
          ["Integrate", "Aggregation over fibers (GROUP BY equivalent)"],
          ["Morphism", "Structure-preserving map between bundles (foreign key equivalent)"],
          ["Projection (π)", "The map from total space to base space (record → key)"],
          ["Pullback", "Categorical pullback join — O(|left|) via hash lookup"],
          ["Redefine", "Update a section's fiber values (UPDATE equivalent)"],
          ["Retract", "Remove a section from the bundle (DELETE equivalent)"],
          ["Section (σ)", "A record — a mapping σ: B → E assigning fiber values to a base point"],
          ["Sheaf Gluing", "The axiom that locally consistent data composes to globally consistent results"],
          ["Spectral Gap (λ₁)", "Smallest nonzero eigenvalue of the graph Laplacian — measures connectivity"],
          ["Stream", "The GIGI server process (gigi-stream) serving REST, WebSocket, and GQL"],
          ["Total Space (E)", "The set of all records in a bundle"],
          ["WAL", "Write-Ahead Log — binary log ensuring crash recovery and durability"],
          ["Zero Section (σ₀)", "The default section where every field has its DEFAULT value"],
        ]}
      />
    </Section>
  );
}

function RecursiveSection() {
  return (
    <>
      <H3>Recursive Queries (ITERATE)</H3>
      <P>GQL supports recursive traversal via ITERATE — walking a graph structure by following field references.</P>
      <Code>{`-- Walk the org chart from employee #1 up the manager chain
ITERATE employees START AT id=1 STEP ALONG manager_id UNTIL VOID;

-- Limit depth
ITERATE employees START AT id=1 STEP ALONG manager_id UNTIL DEPTH 3;

-- With max depth safety
ITERATE employees START AT id=1 STEP ALONG manager_id MAX DEPTH 10;`}</Code>
    </>
  );
}

function CommentsSection() {
  return (
    <>
      <H3>Schema Comments</H3>
      <Code>{`COMMENT ON BUNDLE sensors IS 'NASA POWER atmospheric data';
COMMENT ON FIELD sensors.temp IS 'Temperature at 2 meters (°C)';
SHOW COMMENTS ON sensors;`}</Code>
    </>
  );
}

function InfoSchemaSection() {
  return (
    <>
      <H3>Information Schema</H3>
      <Code>{`SHOW BUNDLES;              -- list all bundles
SHOW FIELDS ON sensors;    -- field list with types
SHOW INDEXES ON sensors;   -- index information
SHOW CONSTRAINTS ON sensors;
SHOW MORPHISMS ON sensors; -- foreign key equivalents
SHOW TRIGGERS ON sensors;
SHOW POLICIES ON sensors;
SHOW STATISTICS ON sensors;
SHOW GEOMETRY ON sensors;  -- curvature, spectral, capacity
SHOW COMMENTS ON sensors;
SHOW ROLES;
SHOW PREPARED;
SHOW BACKUPS;
SHOW SETTINGS;
SHOW SESSION;
SHOW CURRENT ROLE;`}</Code>
    </>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  MAIN APP
// ═══════════════════════════════════════════════════════════════════
export default function GQLDocs() {
  const [active, setActive] = useState("overview");
  const [search, setSearch] = useState("");
  const [sideOpen, setSideOpen] = useState(true);
  const contentRef = useRef(null);

  const scrollTo = (id) => {
    setActive(id);
    const el = document.getElementById(id);
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  // Track active section on scroll
  useEffect(() => {
    const handler = () => {
      const ids = NAV.map(n => n.id);
      for (let i = ids.length - 1; i >= 0; i--) {
        const el = document.getElementById(ids[i]);
        if (el && el.getBoundingClientRect().top <= 120) {
          setActive(ids[i]);
          break;
        }
      }
    };
    window.addEventListener("scroll", handler, true);
    return () => window.removeEventListener("scroll", handler, true);
  }, []);

  const filtered = search
    ? NAV.filter(n => n.label.toLowerCase().includes(search.toLowerCase()))
    : NAV;

  return (
    <div style={{ display: "flex", minHeight: "100vh", background: BG, fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" }}>
      {/* ─── Sidebar ─── */}
      <aside style={{
        width: sideOpen ? 260 : 0,
        minHeight: "100vh",
        background: "#08090E",
        borderRight: `1px solid ${BORDER}`,
        position: "fixed",
        top: 0,
        left: 0,
        bottom: 0,
        overflowY: "auto",
        overflowX: "hidden",
        transition: "width 0.2s",
        zIndex: 200,
      }}>
        <div style={{ padding: "20px 16px 12px" }}>
          <a href={import.meta.env.DEV ? "http://localhost:5176" : "/gigi"} style={{ textDecoration: "none", display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <span style={{ fontSize: 20, fontWeight: 900, color: G }}>GIGI</span>
            <span style={{ fontSize: 11, color: "#404860", fontFamily: MONO }}>docs</span>
          </a>
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search docs..."
            style={{ width: "100%", padding: "8px 10px", background: "#0C0E16", border: `1px solid ${BORDER}`, borderRadius: 6, color: "#C0C8D4", fontSize: 12, fontFamily: MONO, outline: "none" }}
          />
        </div>
        <nav style={{ padding: "0 8px 20px" }}>
          {filtered.map(item => (
            <button
              key={item.id}
              onClick={() => scrollTo(item.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                width: "100%",
                padding: "7px 12px",
                background: active === item.id ? "rgba(64,232,160,0.08)" : "transparent",
                border: "none",
                borderRadius: 6,
                borderLeft: active === item.id ? `2px solid ${G}` : "2px solid transparent",
                cursor: "pointer",
                color: active === item.id ? G : "#606878",
                fontSize: 12.5,
                fontWeight: active === item.id ? 600 : 400,
                textAlign: "left",
                transition: "all 0.15s",
              }}
            >
              <span style={{ fontSize: 13, width: 20, textAlign: "center" }}>{item.icon}</span>
              {item.label}
            </button>
          ))}
        </nav>
      </aside>

      {/* ─── Toggle button ─── */}
      <button
        onClick={() => setSideOpen(!sideOpen)}
        style={{ position: "fixed", top: 12, left: sideOpen ? 265 : 5, zIndex: 300, background: "#0C0E16", border: `1px solid ${BORDER}`, borderRadius: 4, color: "#606878", fontSize: 14, cursor: "pointer", padding: "4px 8px", transition: "left 0.2s" }}
      >
        {sideOpen ? "◀" : "▶"}
      </button>

      {/* ─── Main Content ─── */}
      <main
        ref={contentRef}
        style={{
          marginLeft: sideOpen ? 260 : 0,
          flex: 1,
          padding: "40px 48px 100px",
          maxWidth: 920,
          transition: "margin-left 0.2s",
        }}
      >
        {/* Title */}
        <div style={{ marginBottom: 48 }}>
          <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.2em", color: "#40E8A028", fontFamily: MONO, marginBottom: 8 }}>REFERENCE DOCUMENTATION</div>
          <h1 style={{ fontSize: 42, fontWeight: 900, color: "#E0E8F0", margin: 0, letterSpacing: "-0.03em" }}>
            GQL <span style={{ color: G }}>Reference</span>
          </h1>
          <p style={{ fontSize: 15, color: "#607080", marginTop: 12, lineHeight: 1.65, maxWidth: 600 }}>
            Complete reference for the Geometric Query Language — data types, statements, operators, functions, REST API, WebSocket protocol, JavaScript SDK, and mathematical foundations.
          </p>
          <div style={{ display: "flex", gap: 8, marginTop: 16, flexWrap: "wrap" }}>
            <span style={{ fontSize: 10, padding: "3px 10px", background: "rgba(64,232,160,0.08)", border: `1px solid ${G}33`, borderRadius: 12, color: G, fontFamily: MONO }}>v2.1</span>
            <span style={{ fontSize: 10, padding: "3px 10px", background: "rgba(59,130,246,0.08)", border: "1px solid rgba(59,130,246,0.2)", borderRadius: 12, color: "#3B82F6", fontFamily: MONO }}>39 REST endpoints</span>
            <span style={{ fontSize: 10, padding: "3px 10px", background: "rgba(245,158,11,0.08)", border: "1px solid rgba(245,158,11,0.2)", borderRadius: 12, color: "#F59E0B", fontFamily: MONO }}>Patent Pending</span>
          </div>
        </div>

        <OverviewSection />
        <QuickStartSection />
        <ConceptsSection />
        <TypesSection />
        <BundlesSection />
        <SectionsSection />
        <QueriesSection />
        <FiltersSection />
        <AggregationSection />
        <JoinsSection />
        <TransactionsSection />
        <GeometricSection />
        <SQLSection />
        <AccessControlSection />
        <ConstraintsSection />
        <IndexesSection />
        <PreparedSection />
        <TriggersSection />
        <MaintenanceSection />
        <BackupSection />
        <ImportExportSection />
        <EncryptionSection />
        <RESTSection />
        <WebSocketSection />
        <SDKSection />
        <EdgeSection />
        <ConfigSection />
        <FunctionsSection />
        <ReservedWordsSection />
        <MathSection />
        <ErrorsSection />
        <GlossarySection />

        {/* Information Schema & extras at the end */}
        <Section id="appendix" title="Appendix">
          <RecursiveSection />
          <CommentsSection />
          <InfoSchemaSection />

          <H3>GQL Comment Syntax</H3>
          <Code>{`-- This is a line comment (double-dash)
COVER sensors ALL; -- inline comment`}</Code>

          <H3>Statement Terminator</H3>
          <P>All GQL statements are terminated with a semicolon <code style={{ fontSize: 12, background: "#0E1020", padding: "2px 6px", borderRadius: 3, fontFamily: MONO }}>;</code></P>

          <H3>Identifier Rules</H3>
          <P>Bundle and field names must match <code style={{ fontSize: 12, background: "#0E1020", padding: "2px 6px", borderRadius: 3, fontFamily: MONO }}>[A-Za-z_][A-Za-z0-9_]*</code>. Reserved words cannot be used as unquoted identifiers.</P>
        </Section>

        {/* Footer */}
        <div style={{ marginTop: 80, padding: "24px 0", borderTop: `1px solid ${BORDER}`, textAlign: "center" }}>
          <span style={{ fontSize: 11, color: "#282840", fontFamily: MONO }}>GIGI · Geometric Intrinsic Global Index · U.S. Provisional Application No. 64/008,940 · Davis Geometric Research</span>
        </div>
      </main>
    </div>
  );
}
