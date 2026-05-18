/**
 * Workflow templates — one-click starters that match classic Airtable
 * use cases (project tracker, content calendar, CRM, etc.) but ship
 * with GIGI's geometric substrate (κ overlay, Prism wire-up, field
 * encryption) on day one.
 *
 * Each template is `{ schema, seed_csv, defaultView, prismWireUp }` packaged
 * so the picker can hand it to the apply-workflow handler and the user
 * lands on a usable workspace in a single click.
 *
 * Reusing the "workflow" term that Prism already uses is intentional —
 * the substrate is the same. Prism workflows operate on bundles;
 * these workflows ship pre-baked bundles that Prism workflows can run
 * against immediately.
 *
 * Seed data is intentionally fictitious (Acme / Globex / etc.) so
 * nothing real leaks; row counts are tight (~20-30) to keep the
 * initial ingest snappy.
 */

import type { InferredType } from "./csv";

export type ViewKindForWorkflow =
  | "grid"
  | "kanban"
  | "gallery"
  | "form"
  | "calendar"
  | "gql";

export type EngineFieldType = "text" | "numeric" | "boolean" | "categorical" | "timestamp";

/** Allowed field types — same union the demo loader uses. */
const TYPE_MAP: Record<InferredType, EngineFieldType> = {
  text: "text",
  numeric: "numeric",
  boolean: "boolean",
  categorical: "categorical",
  timestamp: "timestamp",
};
export { TYPE_MAP as WORKFLOW_TYPE_MAP };

export interface WorkflowBundleSpec {
  /** Bundle name on the engine. Use the convention `workflow_<slug>`. */
  name: string;
  /** Map of column name → engine type. Order is preserved for the CSV. */
  fields: Record<string, EngineFieldType>;
  /** Primary key column(s). */
  keys: string[];
  /** CSV body (no header — header is derived from `fields`). */
  seedCsv: string;
  /** Default cover field used by κ + view-tab grouping. */
  suggestedCover: string;
}

export interface WorkflowTemplate {
  /** Slug used everywhere. URL-safe, lowercase, underscores. */
  id: string;
  title: string;
  /** Single emoji for the picker card. */
  icon: string;
  /** One-line pitch for the picker card. */
  blurb: string;
  /** Two- or three-sentence pitch shown on hover / detail. */
  pitch: string;
  /** One or more bundles to create. CRM is two-bundle; the rest are single. */
  bundles: WorkflowBundleSpec[];
  /** Which bundle to open after creation (must match `bundles[i].name`). */
  defaultBundle: string;
  /** Which view to open on the default bundle. */
  defaultView: ViewKindForWorkflow;
  /** Bullet list of Prism workflows this template pairs with. */
  prismWireUp: string[];
  /** The "Airtable can't do this" callout for the workflow card. */
  gigiBetter: string;
}

/* ════════════════════════════════════════════════════════════════════
 * 1 · Project tracker
 * ════════════════════════════════════════════════════════════════════ */
const PROJECT_TRACKER: WorkflowTemplate = {
  id: "project_tracker",
  title: "Project tracker",
  icon: "📋",
  blurb: "Tasks · assignees · statuses · due dates. Kanban-first.",
  pitch:
    "Sprint board for small teams. Tasks group by status on the kanban; due dates light up on the calendar; high-curvature rows surface stalled tickets before standup.",
  bundles: [
    {
      name: "workflow_projects",
      keys: ["task_id"],
      suggestedCover: "status",
      fields: {
        task_id: "text",
        title: "text",
        assignee: "categorical",
        status: "categorical",
        priority: "categorical",
        due_date: "timestamp",
        created_at: "timestamp",
        estimate_hrs: "numeric",
        actual_hrs: "numeric",
      },
      seedCsv: `T-001,Build login flow,bee,in-progress,P0,2026-05-22,2026-05-10,8,5
T-002,Wire Stripe checkout,alex,done,P0,2026-05-15,2026-05-08,12,11
T-003,Pricing page A/B test,jamie,review,P1,2026-05-20,2026-05-09,6,7
T-004,Move logs to ClickHouse,sam,backlog,P2,2026-06-05,2026-05-14,20,0
T-005,Magic-link rate limit,bee,done,P0,2026-05-12,2026-05-07,4,3
T-006,Marketing site copy review,riley,in-progress,P2,2026-05-25,2026-05-11,5,2
T-007,Dashboard load perf,alex,backlog,P1,2026-05-28,2026-05-13,16,0
T-008,Onboarding email sequence,jamie,review,P2,2026-05-21,2026-05-10,8,9
T-009,SOC2 audit prep,sam,in-progress,P0,2026-05-30,2026-05-09,40,28
T-010,Browser extension v2,alex,backlog,P3,2026-06-15,2026-05-14,32,0
T-011,Customer interview cadence,bee,done,P1,2026-05-14,2026-05-08,3,3
T-012,Refresh icon set,riley,done,P2,2026-05-13,2026-05-09,4,5
T-013,Refactor auth middleware,sam,review,P0,2026-05-26,2026-05-12,18,22
T-014,Add audit log UI,alex,backlog,P1,2026-06-02,2026-05-14,10,0
T-015,Quarterly metrics deck,bee,in-progress,P1,2026-05-23,2026-05-11,6,4
T-016,API key rotation flow,jamie,review,P0,2026-05-24,2026-05-10,8,11
T-017,Sales handoff doc,riley,done,P2,2026-05-15,2026-05-08,2,2
T-018,Postgres → bundle migration,sam,backlog,P0,2026-06-08,2026-05-13,24,0
T-019,Mobile responsive sweep,alex,in-progress,P2,2026-05-27,2026-05-12,12,7
T-020,Customer support macros,jamie,done,P3,2026-05-16,2026-05-09,3,3
T-021,Help center search index,bee,review,P1,2026-05-22,2026-05-11,8,9
T-022,Pricing page hero rework,riley,in-progress,P1,2026-05-26,2026-05-13,5,3
T-023,Internal ops dashboard,sam,backlog,P2,2026-06-12,2026-05-14,16,0
T-024,Compliance training rollout,bee,done,P2,2026-05-14,2026-05-09,4,4
T-025,Q3 OKRs draft,alex,review,P1,2026-05-29,2026-05-12,6,5
`,
    },
  ],
  defaultBundle: "workflow_projects",
  defaultView: "kanban",
  prismWireUp: [
    "Monitor flags stalled tasks where actual_hrs / estimate_hrs sits far from cohort",
    "Dedup catches duplicate ticket filings (canonical title match)",
  ],
  gigiBetter:
    "Stalled tasks surface as κ-drift on the work-rate axis — no Zap needed. Same data, fewer plug-ins.",
};

/* ════════════════════════════════════════════════════════════════════
 * 2 · Content calendar
 * ════════════════════════════════════════════════════════════════════ */
const CONTENT_CALENDAR: WorkflowTemplate = {
  id: "content_calendar",
  title: "Content calendar",
  icon: "📅",
  blurb: "Posts · channels · publish dates. Calendar-first.",
  pitch:
    "Editorial calendar for a publishing team. Posts arrange themselves by publish date; each day tints by mean κ — unusually heavy publishing days light up before they ship.",
  bundles: [
    {
      name: "workflow_content_calendar",
      keys: ["post_id"],
      suggestedCover: "channel",
      fields: {
        post_id: "text",
        title: "text",
        channel: "categorical",
        author: "categorical",
        publish_date: "timestamp",
        status: "categorical",
        word_count: "numeric",
        target_audience: "text",
      },
      seedCsv: `P-001,Why we built GIGI Sheets,blog,bee,2026-05-20,scheduled,1850,founders
P-002,The Davis identity in 60 seconds,twitter,bee,2026-05-21,published,280,builders
P-003,Field-level encryption explainer,blog,sam,2026-05-22,draft,2100,enterprise
P-004,Prism Books changelog 0.3,newsletter,jamie,2026-05-23,scheduled,650,users
P-005,LinkedIn: anomaly detection demo,linkedin,alex,2026-05-24,scheduled,420,decision-makers
P-006,Series A announcement,blog,bee,2026-05-25,draft,1450,press
P-007,GQL vs SQL — when each wins,blog,sam,2026-05-26,scheduled,2400,engineers
P-008,Twitter thread: drag-fill demo,twitter,riley,2026-05-27,published,310,users
P-009,Customer story · Cascade Bank,blog,jamie,2026-05-28,scheduled,1800,enterprise
P-010,Engineering blog · embedding internals,blog,sam,2026-05-29,draft,2800,engineers
P-011,Newsletter · May digest,newsletter,bee,2026-05-30,scheduled,520,users
P-012,LinkedIn · pricing rationale,linkedin,bee,2026-05-31,draft,380,decision-makers
P-013,Blog · why we ditched VLOOKUP,blog,alex,2026-06-01,scheduled,1650,users
P-014,Twitter · Dedup before/after,twitter,riley,2026-06-02,draft,290,builders
P-015,Blog · κ-overlay deep dive,blog,sam,2026-06-03,draft,2200,engineers
P-016,Newsletter · feature drops,newsletter,jamie,2026-06-05,scheduled,580,users
P-017,LinkedIn · Series A retrospective,linkedin,bee,2026-06-06,draft,420,press
P-018,Blog · formula bar primitives,blog,sam,2026-06-08,draft,1950,engineers
P-019,Twitter · sameness-join demo,twitter,alex,2026-06-09,draft,265,builders
P-020,Blog · 6 workflows we shipped,blog,bee,2026-06-10,draft,2150,users
`,
    },
  ],
  defaultBundle: "workflow_content_calendar",
  defaultView: "calendar",
  prismWireUp: [
    "Forecast projects posts/week to spot publishing-cadence drops",
    "Dedup on canonical title catches near-duplicate post drafts",
  ],
  gigiBetter:
    "Calendar tints each day by mean κ — three unusually long posts queued for a Tuesday glow yellow before publication.",
};

/* ════════════════════════════════════════════════════════════════════
 * 3 · CRM (two bundles — contacts + deals)
 * ════════════════════════════════════════════════════════════════════ */
const CRM: WorkflowTemplate = {
  id: "crm",
  title: "CRM",
  icon: "💼",
  blurb: "Contacts + Deals (linked). Pipeline kanban by stage.",
  pitch:
    "Sales pipeline for small teams. Contacts and deals link via sameness-join — typo'd email addresses still match, so no leads orphan. Stale deals surface as κ-drift.",
  bundles: [
    {
      name: "workflow_crm_contacts",
      keys: ["contact_id"],
      suggestedCover: "company",
      fields: {
        contact_id: "text",
        name: "text",
        email: "text",
        company: "categorical",
        role: "text",
        last_contacted: "timestamp",
      },
      seedCsv: `C-001,Eleanor Chen,eleanor.chen@acme.com,Acme,VP Engineering,2026-05-12
C-002,Marcus Webb,m.webb@globex.io,Globex,CFO,2026-05-10
C-003,Priya Nair,priya@initech.com,Initech,Head of Data,2026-05-14
C-004,Diego Alvarez,d.alvarez@umbrella.co,Umbrella,COO,2026-05-08
C-005,Hannah Kim,hannah.kim@hooli.com,Hooli,Director of Ops,2026-05-13
C-006,Jamal Okafor,jamal@cyberdyne.io,Cyberdyne,CTO,2026-05-11
C-007,Sofia Romano,sofia@stark.industries,Stark,VP Product,2026-05-09
C-008,Theo Lindqvist,theo.l@piedpiper.com,Pied Piper,Head of BD,2026-05-15
C-009,Aisha Mwangi,aisha@vandelay.com,Vandelay,CEO,2026-05-07
C-010,Yuki Tanaka,yuki@kahnindustries.co,Kahn Industries,VP Finance,2026-05-12
C-011,Kenji Watanabe,kenji@cogswells.com,Cogswells,Director of Eng,2026-05-14
C-012,Beatriz Santos,beatriz@spacely.io,Spacely,COO,2026-05-10
C-013,Mira Chen,mira@dunder-mifflin.com,Dunder Mifflin,VP Sales,2026-05-13
C-014,Issa Diallo,issa@nakatomi.co,Nakatomi,Head of IT,2026-05-08
C-015,Reese Cooper,reese@oscorp.io,Oscorp,Director of Data,2026-05-15
C-016,Magnus Hansen,magnus@tyrell.com,Tyrell,CFO,2026-05-11
C-017,Anya Volkov,anya@weyland.com,Weyland,VP Marketing,2026-05-09
C-018,Tyrell Brooks,tyrell@aperture.io,Aperture,Head of Research,2026-05-14
C-019,Linnea Bergman,linnea@blackmesa.co,Black Mesa,COO,2026-05-12
C-020,Felipe Cardoso,felipe@vaultec.com,Vault-Tec,VP Engineering,2026-05-10
`,
    },
    {
      name: "workflow_crm_deals",
      keys: ["deal_id"],
      suggestedCover: "stage",
      fields: {
        deal_id: "text",
        contact_id: "text",
        stage: "categorical",
        value_usd: "numeric",
        probability_pct: "numeric",
        expected_close: "timestamp",
        notes: "text",
      },
      seedCsv: `D-001,C-001,proposal,125000,60,2026-06-15,Decision-maker engaged, security review pending
D-002,C-002,closed-won,85000,100,2026-05-09,Three-year contract closed
D-003,C-003,qualified,210000,40,2026-07-01,Enterprise deal, security review starting
D-004,C-004,lead,45000,20,2026-07-15,Inbound from blog post
D-005,C-005,proposal,165000,55,2026-06-20,Pricing meeting scheduled
D-006,C-006,closed-lost,75000,0,2026-05-08,Went with incumbent
D-007,C-007,qualified,95000,45,2026-06-25,Trial extended
D-008,C-008,lead,60000,15,2026-07-30,Demo scheduled
D-009,C-009,proposal,140000,65,2026-06-18,Verbal yes, contract redlining
D-010,C-010,closed-won,55000,100,2026-05-10,Annual plan
D-011,C-011,qualified,185000,50,2026-07-05,POC starting
D-012,C-012,lead,40000,25,2026-08-01,First call done
D-013,C-013,proposal,110000,70,2026-06-12,Final approval pending
D-014,C-014,qualified,135000,40,2026-07-08,Solution review next week
D-015,C-015,closed-won,72000,100,2026-05-11,Quarterly plan
D-016,C-016,lead,90000,20,2026-08-10,Inbound demo
D-017,C-017,proposal,155000,60,2026-06-22,Security questionnaire returned
D-018,C-018,qualified,175000,45,2026-07-10,Technical evaluation
D-019,C-019,lead,50000,15,2026-08-05,Cold outbound
D-020,C-020,proposal,120000,55,2026-06-28,Pricing approval pending
`,
    },
  ],
  defaultBundle: "workflow_crm_deals",
  defaultView: "kanban",
  prismWireUp: [
    "Books reconciles deals across CRM ↔ accounting via sameness-join",
    "Monitor flags stale deals — no movement in 14+ days lights up as κ-drift",
  ],
  gigiBetter:
    "Contact IDs link via sameness-join, not strict-FK — a typo'd 'C-O02' still finds 'C-002', so no leads orphan.",
};

/* ════════════════════════════════════════════════════════════════════
 * 4 · Event planning
 * ════════════════════════════════════════════════════════════════════ */
const EVENT_PLANNING: WorkflowTemplate = {
  id: "event_planning",
  title: "Event planning",
  icon: "🎉",
  blurb: "RSVPs · dietary · seating. Form-first intake.",
  pitch:
    "Wedding / conference / dinner workhorse. Form view collects RSVPs; Kanban groups by response; the email column ships OPAQUE-encrypted so organizers query by name without seeing addresses.",
  bundles: [
    {
      name: "workflow_event_rsvps",
      keys: ["attendee_id"],
      suggestedCover: "rsvp",
      fields: {
        attendee_id: "text",
        name: "text",
        email: "text",
        rsvp: "categorical",
        dietary: "text",
        arrival_date: "timestamp",
        table_assignment: "text",
        plus_one: "boolean",
      },
      seedCsv: `A-001,Eleanor Chen,eleanor@example.com,yes,vegetarian,2026-08-14,Table 3,true
A-002,Marcus Webb,marcus@example.com,yes,none,2026-08-14,Table 5,false
A-003,Priya Nair,priya@example.com,maybe,vegan,2026-08-14,unassigned,false
A-004,Diego Alvarez,diego@example.com,yes,gluten-free,2026-08-14,Table 2,true
A-005,Hannah Kim,hannah@example.com,no,,,unassigned,false
A-006,Jamal Okafor,jamal@example.com,yes,none,2026-08-14,Table 5,true
A-007,Sofia Romano,sofia@example.com,pending,,,unassigned,false
A-008,Theo Lindqvist,theo@example.com,yes,pescatarian,2026-08-14,Table 4,true
A-009,Aisha Mwangi,aisha@example.com,yes,none,2026-08-14,Table 1,false
A-010,Yuki Tanaka,yuki@example.com,maybe,kosher,2026-08-15,unassigned,true
A-011,Kenji Watanabe,kenji@example.com,yes,none,2026-08-14,Table 6,false
A-012,Beatriz Santos,beatriz@example.com,yes,vegetarian,2026-08-14,Table 3,true
A-013,Mira Chen,mira@example.com,no,,,unassigned,false
A-014,Issa Diallo,issa@example.com,yes,halal,2026-08-14,Table 7,true
A-015,Reese Cooper,reese@example.com,pending,,,unassigned,false
A-016,Magnus Hansen,magnus@example.com,yes,none,2026-08-14,Table 2,false
A-017,Anya Volkov,anya@example.com,yes,vegetarian,2026-08-14,Table 6,true
A-018,Tyrell Brooks,tyrell@example.com,maybe,none,2026-08-15,unassigned,false
A-019,Linnea Bergman,linnea@example.com,yes,gluten-free,2026-08-14,Table 1,true
A-020,Felipe Cardoso,felipe@example.com,yes,none,2026-08-14,Table 4,false
A-021,Olaf Brennan,olaf@example.com,yes,none,2026-08-14,Table 7,true
A-022,Naomi Park,naomi@example.com,pending,vegan,,unassigned,false
A-023,Hassan El-Sayed,hassan@example.com,yes,halal,2026-08-14,Table 5,false
A-024,Camille Dubois,camille@example.com,yes,none,2026-08-14,Table 3,true
A-025,Bilal Rahman,bilal@example.com,yes,halal,2026-08-14,Table 5,true
A-026,Saoirse Murphy,saoirse@example.com,maybe,pescatarian,2026-08-15,unassigned,false
A-027,Hiroto Sato,hiroto@example.com,yes,none,2026-08-14,Table 6,false
A-028,Mei Lin,mei@example.com,yes,vegetarian,2026-08-14,Table 2,true
A-029,Ravi Subramanian,ravi@example.com,yes,vegetarian,2026-08-14,Table 7,false
A-030,Carlos Rivera,carlos@example.com,no,,,unassigned,false
`,
    },
  ],
  defaultBundle: "workflow_event_rsvps",
  defaultView: "form",
  prismWireUp: [
    "Dedup on canonical (name, email) catches double-RSVPs from different email formats",
  ],
  gigiBetter:
    "Email column ships OPAQUE-encrypted — query by name without ever seeing raw addresses in the grid.",
};

/* ════════════════════════════════════════════════════════════════════
 * 5 · Inventory management
 * ════════════════════════════════════════════════════════════════════ */
const INVENTORY: WorkflowTemplate = {
  id: "inventory",
  title: "Inventory",
  icon: "📦",
  blurb: "SKUs · suppliers · stock levels. Reorder alerts built in.",
  pitch:
    "Stock-take spreadsheet that grew up. Filter to under-threshold rows in one click; Forecast tells you which SKUs hit reorder by day 7; Monitor flags unusual stock velocity.",
  bundles: [
    {
      name: "workflow_inventory",
      keys: ["sku"],
      suggestedCover: "category",
      fields: {
        sku: "text",
        product_name: "text",
        category: "categorical",
        supplier_id: "categorical",
        quantity_on_hand: "numeric",
        reorder_threshold: "numeric",
        unit_cost_usd: "numeric",
        last_restocked: "timestamp",
      },
      seedCsv: `SKU-001,Wireless mouse,electronics,SUP-A,142,50,18.50,2026-05-01
SKU-002,USB-C cable 2m,electronics,SUP-A,38,80,7.20,2026-04-22
SKU-003,Mechanical keyboard,electronics,SUP-B,67,30,92.00,2026-05-04
SKU-004,Yoga mat 6mm,sports,SUP-C,225,100,22.50,2026-05-02
SKU-005,Resistance bands set,sports,SUP-C,18,40,14.80,2026-04-18
SKU-006,Stainless water bottle,home,SUP-D,310,80,12.40,2026-05-03
SKU-007,Bamboo cutting board,home,SUP-D,85,50,28.00,2026-04-28
SKU-008,Ceramic mug 12oz,home,SUP-E,420,150,5.80,2026-05-05
SKU-009,Cotton t-shirt L,apparel,SUP-F,165,60,8.50,2026-05-01
SKU-010,Cotton t-shirt M,apparel,SUP-F,148,60,8.50,2026-05-01
SKU-011,Cotton t-shirt S,apparel,SUP-F,22,60,8.50,2026-05-01
SKU-012,Hoodie XL black,apparel,SUP-F,85,40,32.00,2026-04-25
SKU-013,Webcam 1080p,electronics,SUP-B,52,40,45.00,2026-05-06
SKU-014,Desk lamp LED,home,SUP-E,118,50,38.00,2026-05-04
SKU-015,Running socks 3-pack,apparel,SUP-G,240,80,11.20,2026-05-02
SKU-016,Bluetooth speaker,electronics,SUP-A,8,30,55.00,2026-04-20
SKU-017,Yoga block foam,sports,SUP-C,140,50,8.20,2026-05-03
SKU-018,Spice rack 16-jar,home,SUP-D,32,20,42.00,2026-04-27
SKU-019,Phone tripod,electronics,SUP-A,95,40,24.00,2026-05-01
SKU-020,Foam roller,sports,SUP-C,68,30,28.50,2026-05-04
SKU-021,Linen napkins 4-pack,home,SUP-E,182,60,18.00,2026-05-02
SKU-022,Beanie wool grey,apparel,SUP-G,76,40,16.50,2026-04-26
SKU-023,Notebook A5 lined,home,SUP-H,265,100,9.80,2026-05-03
SKU-024,Gel pen black 12-pack,home,SUP-H,12,80,6.40,2026-04-19
SKU-025,Backpack 30L,apparel,SUP-G,48,30,68.00,2026-04-30
`,
    },
  ],
  defaultBundle: "workflow_inventory",
  defaultView: "grid",
  prismWireUp: [
    "Forecast projects per-SKU stock-out timing",
    "Monitor flags unusual stock-velocity rows as κ-anomalies",
  ],
  gigiBetter:
    "Forecast tells you which SKUs will hit reorder-threshold by day 7 — with a √step confidence band — no plug-in needed.",
};

/* ════════════════════════════════════════════════════════════════════
 * 6 · Recruiting pipeline
 * ════════════════════════════════════════════════════════════════════ */
const RECRUITING: WorkflowTemplate = {
  id: "recruiting",
  title: "Recruiting pipeline",
  icon: "👥",
  blurb: "Candidates · stages · skills · scores. Kanban by stage.",
  pitch:
    "ATS-shaped pipeline. Candidates flow through stages on the kanban; sameness-find surfaces 'candidates like this hired one'; Monitor flags top-scored rejected applicants as likely false negatives.",
  bundles: [
    {
      name: "workflow_recruiting",
      keys: ["applicant_id"],
      suggestedCover: "stage",
      fields: {
        applicant_id: "text",
        full_name: "text",
        role: "categorical",
        stage: "categorical",
        top_skill: "categorical",
        experience_years: "numeric",
        location: "text",
        salary_ask_usd: "numeric",
        score: "numeric",
        applied_date: "timestamp",
      },
      seedCsv: `R-001,Eleanor Chen,Software Engineer,onsite,Rust,7,Remote,185000,8.7,2026-05-02
R-002,Marcus Webb,Sales,phone-screen,Enterprise,9,Chicago,200000,8.0,2026-05-03
R-003,Priya Nair,Data Scientist,offer,Python,6,New York,175000,8.9,2026-05-04
R-004,Diego Alvarez,Software Engineer,applied,Go,4,Mexico City,135000,7.3,2026-05-05
R-005,Hannah Kim,Software Engineer,hired,TypeScript,8,Vancouver,180000,9.0,2026-05-06
R-006,Jamal Okafor,Product Manager,onsite,Strategy,7,Remote,195000,8.6,2026-05-07
R-007,Sofia Romano,Designer,phone-screen,Figma,4,Milan,125000,7.4,2026-05-08
R-008,Theo Lindqvist,Designer,offer,Figma,6,Stockholm,150000,8.5,2026-05-09
R-009,Aisha Mwangi,Sales,applied,Enterprise,5,Nairobi,140000,7.6,2026-05-10
R-010,Yuki Tanaka,Data Scientist,onsite,ML,7,Osaka,170000,8.4,2026-05-11
R-011,Kenji Watanabe,Software Engineer,hired,TypeScript,9,Tokyo,205000,9.1,2026-05-12
R-012,Beatriz Santos,Product Manager,offer,Strategy,8,Lisbon,200000,9.0,2026-05-13
R-013,Mira Chen,Software Engineer,phone-screen,Rust,6,San Francisco,180000,7.9,2026-05-14
R-014,Issa Diallo,Software Engineer,onsite,Rust,6,Dakar,160000,8.2,2026-05-15
R-015,Reese Cooper,Software Engineer,hired,JavaScript,2,Remote,110000,4.9,2026-05-16
R-016,Magnus Hansen,Product Manager,offer,Strategy,9,Copenhagen,215000,9.2,2026-05-17
R-017,Anya Volkov,Sales,hired,Enterprise,11,London,230000,9.0,2026-05-18
R-018,Tyrell Brooks,Software Engineer,onsite,Go,6,Atlanta,170000,8.3,2026-05-19
R-019,Linnea Bergman,Designer,onsite,Figma,7,Gothenburg,165000,8.6,2026-05-20
R-020,Felipe Cardoso,Data Scientist,applied,Python,3,São Paulo,130000,6.8,2026-05-21
`,
    },
  ],
  defaultBundle: "workflow_recruiting",
  defaultView: "kanban",
  prismWireUp: [
    "Monitor flags high-score rejected candidates as likely false negatives",
    "Dedup on canonical (name, email) catches duplicate applications",
  ],
  gigiBetter:
    "'Find candidates like A-005' is one formula: =SAME(R_005, applicant_i) ≥ 0.85. Airtable can't express this without a custom script.",
};

export const WORKFLOW_TEMPLATES: WorkflowTemplate[] = [
  PROJECT_TRACKER,
  CONTENT_CALENDAR,
  CRM,
  EVENT_PLANNING,
  INVENTORY,
  RECRUITING,
];

export function findWorkflowTemplate(id: string): WorkflowTemplate | null {
  return WORKFLOW_TEMPLATES.find((t) => t.id === id) ?? null;
}

/**
 * Compose the full CSV (header + body) for a workflow bundle.
 * The header is derived from the `fields` map's insertion order.
 */
export function workflowCsv(bundle: WorkflowBundleSpec): string {
  const header = Object.keys(bundle.fields).join(",");
  return `${header}\n${bundle.seedCsv}`;
}
