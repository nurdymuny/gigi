/**
 * Plain-English explanations of the geometric terms GIGI uses.
 *
 * Each entry is what we'd say to a non-mathematician operator who clicks
 * the (ⓘ) icon next to a verb or selector. Three parts:
 *   - `summary`   — one-sentence "what is this?"
 *   - `body`      — 2-3 paragraphs of plain-English, how to think about it
 *   - `example`   — a concrete worked example, the kind you'd put on a
 *                   napkin to explain to a coworker
 *   - `gql`       — the GQL statement that runs this verb (optional)
 *
 * Keep the prose human. No math jargon without a translation. The goal is
 * "a smart non-mathematician can decide whether to click the button."
 */

export interface GlossaryEntry {
  /** Display name, e.g. "Cover field" or "SPECTRAL". */
  title: string;
  /** One-sentence plain-English summary. Shown as the subtitle. */
  summary: string;
  /** Longer plain-English explanation, 2-3 short paragraphs. */
  body: string[];
  /** A concrete worked example. */
  example: {
    setup: string;
    result: string;
  };
  /** Optional GQL statement template. */
  gql?: string;
}

export const GLOSSARY: Record<string, GlossaryEntry> = {
  cover: {
    title: "Cover field",
    summary:
      "The column GIGI uses to group your rows into cohorts. Anomalies are measured within each group.",
    body: [
      "Pick a column whose values name a group: region, device_type, customer_tier, day_of_week, etc. GIGI bundles up the rows that share a value into a 'cohort' — a peer group — and asks how far each row is from the center of its own group.",
      "Think of it like grading on a curve. A temperature reading of 28°C is unremarkable in Lagos and a heatwave in Stockholm. The cover field is how you tell GIGI which curve to grade against.",
      "Changing the cover field re-computes κ (the 'weirdness' score) for every row. Numeric columns can't be cover fields — only categorical or text columns make sense as group labels.",
    ],
    example: {
      setup:
        "You have 5,000 sensor readings across 12 cities. Without a cover, every reading is compared to the global average and Lagos gets flagged constantly.",
      result:
        "Set cover = 'city'. Now Lagos readings are compared to other Lagos readings. Only readings that are weird for their own city light up red — which is what you actually care about.",
    },
  },

  kappa: {
    title: "κ (curvature)",
    summary: "How weird this row is compared to its peers in the same cover group.",
    body: [
      "κ measures the distance from a row to the center of its cohort, in the space of its numeric fields. Low κ means 'looks like everyone else in this group'. High κ means 'this one stands out'.",
      "We classify κ in three bands. Green (healthy) means within normal range. Amber (drift) is starting to wander from the cohort center. Red (anomaly) means an outlier worth a human look.",
      "κ̄ (κ-bar) in the top bar is the average across all rows currently in view. A rising κ̄ means the bundle is getting more diffuse — drift is spreading.",
    ],
    example: {
      setup:
        "12 sensors in Lagos averaging 32°C ± 1.5. Sensor S-007 reads 41°C while its peers read 31-33°C.",
      result:
        "S-007 gets κ ≈ 6.0 → flagged red. The other Lagos sensors get κ < 1 → green.",
    },
  },

  curvature: {
    title: "CURVATURE",
    summary:
      "Returns the mean curvature κ̄ across the whole bundle (or a slice).",
    body: [
      "Where κ is per-row, CURVATURE is the aggregate — a single number telling you how 'spread out' the bundle is overall. The GQL view defaults to this query when you open a bundle.",
      "Useful as a top-line health metric. Set up an alert if κ̄ drifts above a threshold for an extended period. Combine with cover slices to ask 'is curvature rising in Lagos specifically?'",
    ],
    example: {
      setup:
        "Daily check on your sensor bundle. κ̄ has hovered at 0.4 for weeks, then suddenly jumps to 1.8.",
      result:
        "Something changed in the data distribution. Drill in by cover field to find which group caused the jump.",
    },
    gql: "CURVATURE <bundle>;",
  },

  section: {
    title: "SECTION",
    summary:
      "Pull a single row (one specific point in the bundle) by its key.",
    body: [
      "SECTION is the O(1) point query — you give it the primary key and it gives you back the row, decrypted as much as your keyring allows. The grid view uses this implicitly when you click a row.",
      "Unlike a SQL SELECT, SECTION is geometric: it returns not just the values but the row's κ, confidence, and capacity — the local geometry around that one point.",
    ],
    example: {
      setup:
        "You want everything GIGI knows about sensor S-042 right now.",
      result:
        "`SECTION sensors AT (sensor_id='S-042')` returns the row plus κ=0.31, confidence=0.76, capacity=6.4.",
    },
    gql: "SECTION <bundle> AT (<key>=<value>);",
  },

  spectral: {
    title: "SPECTRAL",
    summary:
      "The top eigenvalue λ₁ and capacity — how 'wide-open' the bundle's geometry is.",
    body: [
      "SPECTRAL runs an eigendecomposition of the bundle's connectivity. The top eigenvalue λ₁ tells you how strongly rows are clustered. Low λ₁ ≈ tightly knit. High λ₁ ≈ loose, room to grow.",
      "Capacity is derived from λ₁ as a back-of-the-envelope budget — how much you can add to the bundle before geometry starts breaking down. Think of it as 'how many more rows can I fit before things get weird'.",
      "Run this after a big import, or once a week on a healthy bundle to track structural drift.",
    ],
    example: {
      setup:
        "Two warehouses: A has 200 product SKUs tightly clustered by category. B has 200 SKUs scattered across many categories.",
      result:
        "Warehouse A: λ₁ ≈ 0.4, low capacity (filled). Warehouse B: λ₁ ≈ 1.6, higher capacity (room to grow).",
    },
    gql: "SPECTRAL <bundle>;",
  },

  transport: {
    title: "TRANSPORT",
    summary:
      "How much rotation accumulates as you move from row A to row B across the bundle.",
    body: [
      "Parallel transport asks: if I take row A's profile and 'slide' it along the most natural path to row B, how does it rotate? A rotation of 0° means A and B are perfectly aligned. A large rotation means moving between them takes you through significant change.",
      "We use it to compare two rows that aren't directly adjacent — like asking 'how similar is sensor S-001 to sensor S-099, accounting for everything in between?' The dashed line in the scatter view is a TRANSPORT path.",
    ],
    example: {
      setup:
        "Two customer profiles. Alice (S-001) buys mostly books. Carol (S-099) buys mostly cookware. Transport from Alice to Carol.",
      result:
        "Rotation = 0.71 rad (~40°). The path passes through customers who buy both — kitchen-tools-meets-cookbooks intermediates.",
    },
    gql: "TRANSPORT <bundle> FROM (<key>=A) TO (<key>=B) ALONG (<fields>);",
  },

  holonomy: {
    title: "HOLONOMY",
    summary:
      "Walk a closed loop and measure: did you end up facing the same direction?",
    body: [
      "Holonomy is what's left over after you transport around a loop and come back to where you started. In flat geometry, you should return facing the same way — zero holonomy. Any non-zero result is a signal that the loop encloses curvature.",
      "Practically: pick a cover field (a way to group rows), and HOLONOMY walks the cohorts in order, transports through each, and reports the residual rotation. A high holonomy means 'something is bending the geometry inside this loop'.",
      "Useful for spotting cross-group inconsistency. If the loop region/city/team/etc returns non-trivial holonomy, you probably have an outlier group skewing the comparison.",
    ],
    example: {
      setup:
        "Loop around the 'region' field: NA → EU → APAC → NA. In a healthy bundle, the residual rotation should be tiny.",
      result:
        "Holonomy = 0.42 rad means somewhere in NA→EU→APAC the geometry twisted. APAC is probably the culprit — drill in.",
    },
    gql: "HOLONOMY <bundle> AROUND <cover_field>;",
  },

  betti: {
    title: "BETTI",
    summary:
      "Topological 'hole count' — how many disconnected pieces, loops, and voids your data has.",
    body: [
      "Betti numbers are the simplest summary of the shape of your data. b₀ is the number of disconnected blobs (how many separate communities). b₁ is the number of independent loops (recurring cycles). b₂ counts hollow regions.",
      "If b₀ = 1, your bundle is one connected piece. If b₀ = 3, you actually have three sub-populations masquerading as one bundle — likely worth splitting.",
      "Loops (b₁ > 0) often appear in time-series bundles with seasonality, or relational data with cycles (A → B → C → A).",
    ],
    example: {
      setup:
        "You think you have one user base, run BETTI, and get b₀ = 2.",
      result:
        "There are actually two disconnected groups. Likely two distinct customer segments with no overlap — split the bundle and analyze separately.",
    },
    gql: "BETTI <bundle>;",
  },

  integrate: {
    title: "INTEGRATE",
    summary:
      "Add up (or average) a numeric field across a slice of the bundle.",
    body: [
      "INTEGRATE is the geometric version of a SUM or AVG. Give it a field and an optional slice (a cover-field filter or a region in fiber space) and it returns the aggregate.",
      "Different from a plain SUM because it weights by the local geometry — rows in dense regions contribute less per-row than rows in sparse regions. Useful for unbiased aggregates across uneven cohorts.",
    ],
    example: {
      setup:
        "Average temperature across all sensors. A naive average is biased toward Lagos because Lagos has 200 sensors and Reykjavik has 3.",
      result:
        "INTEGRATE temp gives 18.2°C — weighted to give Reykjavik a fair voice. Plain AVG would give 28°C.",
    },
    gql: "INTEGRATE <field> OVER <bundle> [WHERE <cover>=<value>];",
  },

  geodesic: {
    title: "GEODESIC",
    summary: "The shortest natural path from one row to another across the bundle.",
    body: [
      "A geodesic is the geometric 'as the crow flies' between two rows, but the crow has to stay on the manifold — it can't cut through empty data. The result is a sequence of intermediate rows that smoothly connects the two endpoints.",
      "Useful for 'find me the bridge between A and B'. If Alice and Carol don't look similar, the geodesic shows you the customers in between who explain the journey from one to the other.",
    ],
    example: {
      setup:
        "Customer A is a heavy book buyer. Customer B is a heavy cookware buyer. GEODESIC from A to B.",
      result:
        "Returns a path of ~8 intermediate customers: A → cookbooks → kitchen-gift-buyers → casual-cookware → B.",
    },
    gql: "GEODESIC <bundle> FROM (<key>=A) TO (<key>=B);",
  },

  capacity: {
    title: "Capacity C = τ/κ",
    summary:
      "Rough headroom estimate — how many more rows could fit before geometry breaks.",
    body: [
      "Capacity is derived from the spectral gap τ and the local curvature κ. High C means the bundle can absorb more data without losing structure. Low C (or ∞ when κ = 0) means you're already at the limit or trivially uncongested.",
      "Not a hard quota — think of it as a temperature gauge. If C drops fast after each import, your structure is filling up and you might need to split the bundle or add fields.",
    ],
    example: {
      setup:
        "Bundle starts at C = 12.0. After an import of 5,000 rows, C drops to 3.2.",
      result:
        "Headroom shrank to a quarter — geometry is filling up. Either split the bundle by a natural cover or add more fiber dimensions.",
    },
  },

  confidence: {
    title: "Confidence 1/(1+κ)",
    summary:
      "How much to trust this row's local geometry. 1.0 = perfect peer agreement, 0 = total outlier.",
    body: [
      "A simple smoothing of κ into a 0-1 range. Useful when you want a 'should I trust this row?' single number to surface alongside a value.",
      "Sub-0.4 confidence is the same threshold as κ-bad — the engine doesn't trust the geometry here. Use it as a guard in dashboards.",
    ],
    example: {
      setup: "Sensor S-007 reports κ = 6.0.",
      result:
        "Confidence = 1/(1+6) = 0.14. Treat this reading as suspect until it's confirmed.",
    },
  },
};

/** Look up a glossary entry. Case-insensitive, returns null if unknown. */
export function lookupTerm(term: string): GlossaryEntry | null {
  return GLOSSARY[term.toLowerCase()] ?? null;
}
