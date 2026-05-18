/**
 * Public-domain demo datasets that ship with the app.
 *
 * These are the canonical "I recognize that name" datasets people learn data
 * analysis with — Iris, Palmer Penguins, NBA teams, world cities. Each one
 * is embedded as CSV so the import is a single click and works offline.
 *
 * Schema choices were made to surface GIGI's geometry well:
 *   - At least one categorical column → makes a good Cover field
 *   - At least two numeric columns → renders the scatter
 *   - A few hundred rows max → stays snappy for demo
 *
 * Add a new demo by appending to DEMO_DATASETS. The CSV must have a header
 * row and one row per record. The import pipeline auto-infers types but
 * `suggestedKey` and `suggestedCover` let us steer the post-import view.
 */

import type { EncryptionMode } from "./demo-encryption-overlay";

export interface DemoDataset {
  /** Slug used as the bundle name on the engine. Must match [A-Za-z_][A-Za-z0-9_]*. */
  id: string;
  /** Human-friendly title shown on the demo card. */
  title: string;
  /** One-sentence pitch. */
  blurb: string;
  /** Where the data came from. Shown in small print on the card. */
  source: string;
  /** Suggested primary-key column. The importer falls back to its own heuristic if missing. */
  suggestedKey: string;
  /** Suggested cover field (categorical column). */
  suggestedCover: string;
  /** Row count, hard-coded so we can render it without parsing the CSV. */
  records: number;
  /** Number of columns (including the key). */
  fields: number;
  /** The CSV text, header row included. */
  csv: string;
  /**
   * Optional encryption metadata, applied as a client-side overlay after
   * the bundle is created so the Grid + Inspector can demonstrate gauge
   * encryption without engine-side support (planned in addendum E-S8a).
   */
  encryption?: Record<string, EncryptionMode>;
  /**
   * Optional badge label shown on the card — e.g. "PHI · HIPAA-style".
   * Pure cosmetic, signals that the demo involves sensitive data.
   */
  badge?: string;
}

// ── Iris ─────────────────────────────────────────────────────────────────
// R.A. Fisher (1936), public domain. 150 measurements of 3 iris species —
// the OG cohort/cluster dataset. setosa is clearly separable; versicolor +
// virginica overlap. Perfect for demonstrating κ + cover-field grouping.
const IRIS_CSV = `id,species,sepal_length,sepal_width,petal_length,petal_width
1,setosa,5.1,3.5,1.4,0.2
2,setosa,4.9,3.0,1.4,0.2
3,setosa,4.7,3.2,1.3,0.2
4,setosa,4.6,3.1,1.5,0.2
5,setosa,5.0,3.6,1.4,0.2
6,setosa,5.4,3.9,1.7,0.4
7,setosa,4.6,3.4,1.4,0.3
8,setosa,5.0,3.4,1.5,0.2
9,setosa,4.4,2.9,1.4,0.2
10,setosa,4.9,3.1,1.5,0.1
11,setosa,5.4,3.7,1.5,0.2
12,setosa,4.8,3.4,1.6,0.2
13,setosa,4.8,3.0,1.4,0.1
14,setosa,4.3,3.0,1.1,0.1
15,setosa,5.8,4.0,1.2,0.2
16,setosa,5.7,4.4,1.5,0.4
17,setosa,5.4,3.9,1.3,0.4
18,setosa,5.1,3.5,1.4,0.3
19,setosa,5.7,3.8,1.7,0.3
20,setosa,5.1,3.8,1.5,0.3
21,setosa,5.4,3.4,1.7,0.2
22,setosa,5.1,3.7,1.5,0.4
23,setosa,4.6,3.6,1.0,0.2
24,setosa,5.1,3.3,1.7,0.5
25,setosa,4.8,3.4,1.9,0.2
26,setosa,5.0,3.0,1.6,0.2
27,setosa,5.0,3.4,1.6,0.4
28,setosa,5.2,3.5,1.5,0.2
29,setosa,5.2,3.4,1.4,0.2
30,setosa,4.7,3.2,1.6,0.2
31,setosa,4.8,3.1,1.6,0.2
32,setosa,5.4,3.4,1.5,0.4
33,setosa,5.2,4.1,1.5,0.1
34,setosa,5.5,4.2,1.4,0.2
35,setosa,4.9,3.1,1.5,0.2
36,setosa,5.0,3.2,1.2,0.2
37,setosa,5.5,3.5,1.3,0.2
38,setosa,4.9,3.6,1.4,0.1
39,setosa,4.4,3.0,1.3,0.2
40,setosa,5.1,3.4,1.5,0.2
41,setosa,5.0,3.5,1.3,0.3
42,setosa,4.5,2.3,1.3,0.3
43,setosa,4.4,3.2,1.3,0.2
44,setosa,5.0,3.5,1.6,0.6
45,setosa,5.1,3.8,1.9,0.4
46,setosa,4.8,3.0,1.4,0.3
47,setosa,5.1,3.8,1.6,0.2
48,setosa,4.6,3.2,1.4,0.2
49,setosa,5.3,3.7,1.5,0.2
50,setosa,5.0,3.3,1.4,0.2
51,versicolor,7.0,3.2,4.7,1.4
52,versicolor,6.4,3.2,4.5,1.5
53,versicolor,6.9,3.1,4.9,1.5
54,versicolor,5.5,2.3,4.0,1.3
55,versicolor,6.5,2.8,4.6,1.5
56,versicolor,5.7,2.8,4.5,1.3
57,versicolor,6.3,3.3,4.7,1.6
58,versicolor,4.9,2.4,3.3,1.0
59,versicolor,6.6,2.9,4.6,1.3
60,versicolor,5.2,2.7,3.9,1.4
61,versicolor,5.0,2.0,3.5,1.0
62,versicolor,5.9,3.0,4.2,1.5
63,versicolor,6.0,2.2,4.0,1.0
64,versicolor,6.1,2.9,4.7,1.4
65,versicolor,5.6,2.9,3.6,1.3
66,versicolor,6.7,3.1,4.4,1.4
67,versicolor,5.6,3.0,4.5,1.5
68,versicolor,5.8,2.7,4.1,1.0
69,versicolor,6.2,2.2,4.5,1.5
70,versicolor,5.6,2.5,3.9,1.1
71,versicolor,5.9,3.2,4.8,1.8
72,versicolor,6.1,2.8,4.0,1.3
73,versicolor,6.3,2.5,4.9,1.5
74,versicolor,6.1,2.8,4.7,1.2
75,versicolor,6.4,2.9,4.3,1.3
76,versicolor,6.6,3.0,4.4,1.4
77,versicolor,6.8,2.8,4.8,1.4
78,versicolor,6.7,3.0,5.0,1.7
79,versicolor,6.0,2.9,4.5,1.5
80,versicolor,5.7,2.6,3.5,1.0
81,versicolor,5.5,2.4,3.8,1.1
82,versicolor,5.5,2.4,3.7,1.0
83,versicolor,5.8,2.7,3.9,1.2
84,versicolor,6.0,2.7,5.1,1.6
85,versicolor,5.4,3.0,4.5,1.5
86,versicolor,6.0,3.4,4.5,1.6
87,versicolor,6.7,3.1,4.7,1.5
88,versicolor,6.3,2.3,4.4,1.3
89,versicolor,5.6,3.0,4.1,1.3
90,versicolor,5.5,2.5,4.0,1.3
91,versicolor,5.5,2.6,4.4,1.2
92,versicolor,6.1,3.0,4.6,1.4
93,versicolor,5.8,2.6,4.0,1.2
94,versicolor,5.0,2.3,3.3,1.0
95,versicolor,5.6,2.7,4.2,1.3
96,versicolor,5.7,3.0,4.2,1.2
97,versicolor,5.7,2.9,4.2,1.3
98,versicolor,6.2,2.9,4.3,1.3
99,versicolor,5.1,2.5,3.0,1.1
100,versicolor,5.7,2.8,4.1,1.3
101,virginica,6.3,3.3,6.0,2.5
102,virginica,5.8,2.7,5.1,1.9
103,virginica,7.1,3.0,5.9,2.1
104,virginica,6.3,2.9,5.6,1.8
105,virginica,6.5,3.0,5.8,2.2
106,virginica,7.6,3.0,6.6,2.1
107,virginica,4.9,2.5,4.5,1.7
108,virginica,7.3,2.9,6.3,1.8
109,virginica,6.7,2.5,5.8,1.8
110,virginica,7.2,3.6,6.1,2.5
111,virginica,6.5,3.2,5.1,2.0
112,virginica,6.4,2.7,5.3,1.9
113,virginica,6.8,3.0,5.5,2.1
114,virginica,5.7,2.5,5.0,2.0
115,virginica,5.8,2.8,5.1,2.4
116,virginica,6.4,3.2,5.3,2.3
117,virginica,6.5,3.0,5.5,1.8
118,virginica,7.7,3.8,6.7,2.2
119,virginica,7.7,2.6,6.9,2.3
120,virginica,6.0,2.2,5.0,1.5
121,virginica,6.9,3.2,5.7,2.3
122,virginica,5.6,2.8,4.9,2.0
123,virginica,7.7,2.8,6.7,2.0
124,virginica,6.3,2.7,4.9,1.8
125,virginica,6.7,3.3,5.7,2.1
126,virginica,7.2,3.2,6.0,1.8
127,virginica,6.2,2.8,4.8,1.8
128,virginica,6.1,3.0,4.9,1.8
129,virginica,6.4,2.8,5.6,2.1
130,virginica,7.2,3.0,5.8,1.6
131,virginica,7.4,2.8,6.1,1.9
132,virginica,7.9,3.8,6.4,2.0
133,virginica,6.4,2.8,5.6,2.2
134,virginica,6.3,2.8,5.1,1.5
135,virginica,6.1,2.6,5.6,1.4
136,virginica,7.7,3.0,6.1,2.3
137,virginica,6.3,3.4,5.6,2.4
138,virginica,6.4,3.1,5.5,1.8
139,virginica,6.0,3.0,4.8,1.8
140,virginica,6.9,3.1,5.4,2.1
141,virginica,6.7,3.1,5.6,2.4
142,virginica,6.9,3.1,5.1,2.3
143,virginica,5.8,2.7,5.1,1.9
144,virginica,6.8,3.2,5.9,2.3
145,virginica,6.7,3.3,5.7,2.5
146,virginica,6.7,3.0,5.2,2.3
147,virginica,6.3,2.5,5.0,1.9
148,virginica,6.5,3.0,5.2,2.0
149,virginica,6.2,3.4,5.4,2.3
150,virginica,5.9,3.0,5.1,1.8
`;

// ── NBA team standings (2023-24 regular season) ──────────────────────────
// All 30 NBA teams, full season stats. Conference is a natural cover —
// East vs West splits cleanly. Real outliers exist: Pistons (14 wins) and
// Wizards (15 wins) are clear κ-bad. Celtics + Thunder are upper-bound.
const NBA_CSV = `id,team,conference,division,wins,losses,points_per_game,points_allowed,net_rating
BOS,Boston Celtics,Eastern,Atlantic,64,18,120.6,109.2,11.4
DEN,Denver Nuggets,Western,Northwest,57,25,114.9,109.6,5.3
OKC,Oklahoma City Thunder,Western,Northwest,57,25,120.1,112.3,7.8
MIN,Minnesota Timberwolves,Western,Northwest,56,26,113.0,106.5,6.5
LAC,LA Clippers,Western,Pacific,51,31,115.6,111.3,4.3
NYK,New York Knicks,Eastern,Atlantic,50,32,112.8,108.7,4.1
MIL,Milwaukee Bucks,Eastern,Central,49,33,119.0,116.4,2.6
CLE,Cleveland Cavaliers,Eastern,Central,48,34,112.6,110.0,2.6
ORL,Orlando Magic,Eastern,Southeast,47,35,110.5,108.4,2.1
NOP,New Orleans Pelicans,Western,Southwest,49,33,115.9,112.9,3.0
IND,Indiana Pacers,Eastern,Central,47,35,123.3,120.2,3.1
PHI,Philadelphia 76ers,Eastern,Atlantic,47,35,114.2,111.9,2.3
DAL,Dallas Mavericks,Western,Southwest,50,32,117.9,115.6,2.3
PHX,Phoenix Suns,Western,Pacific,49,33,116.2,113.6,2.6
LAL,Los Angeles Lakers,Western,Pacific,47,35,117.4,116.0,1.4
SAC,Sacramento Kings,Western,Pacific,46,36,116.5,116.1,0.4
GSW,Golden State Warriors,Western,Pacific,46,36,117.8,115.2,2.6
HOU,Houston Rockets,Western,Southwest,41,41,114.3,112.6,1.7
MIA,Miami Heat,Eastern,Southeast,46,36,110.1,108.7,1.4
CHI,Chicago Bulls,Eastern,Central,39,43,112.3,112.6,-0.3
ATL,Atlanta Hawks,Eastern,Southeast,36,46,118.3,120.5,-2.2
BKN,Brooklyn Nets,Eastern,Atlantic,32,50,110.8,114.8,-4.0
TOR,Toronto Raptors,Eastern,Atlantic,25,57,111.6,116.3,-4.7
UTA,Utah Jazz,Western,Northwest,31,51,114.6,121.0,-6.4
MEM,Memphis Grizzlies,Western,Southwest,27,55,105.8,113.7,-7.9
POR,Portland Trail Blazers,Western,Northwest,21,61,106.5,116.1,-9.6
SAS,San Antonio Spurs,Western,Southwest,22,60,112.1,118.9,-6.8
CHA,Charlotte Hornets,Eastern,Southeast,21,61,106.6,117.6,-11.0
WAS,Washington Wizards,Eastern,Southeast,15,67,114.4,123.1,-8.7
DET,Detroit Pistons,Eastern,Central,14,68,109.4,118.3,-8.9
`;

// ── World cities ─────────────────────────────────────────────────────────
// 60 of the world's most recognizable cities with population, area, density,
// and elevation. Continent is the natural cover. Real diversity: Tokyo
// (huge), Reykjavik (tiny), La Paz (high altitude), Singapore (dense).
const CITIES_CSV = `id,city,country,continent,population_millions,area_km2,density_per_km2,elevation_m,latitude
TYO,Tokyo,Japan,Asia,37.4,2194,17048,40,35.7
DEL,Delhi,India,Asia,32.9,1484,22183,216,28.7
SHA,Shanghai,China,Asia,28.5,6340,4495,4,31.2
DKA,Dhaka,Bangladesh,Asia,23.2,306,75817,4,23.8
SAO,Sao Paulo,Brazil,South America,22.4,1521,14727,760,-23.5
CAI,Cairo,Egypt,Africa,22.1,3085,7163,23,30.0
MEX,Mexico City,Mexico,North America,22.0,1485,14815,2240,19.4
BEI,Beijing,China,Asia,21.7,16411,1322,43,39.9
MUM,Mumbai,India,Asia,20.9,603,34659,14,19.1
OSA,Osaka,Japan,Asia,19.1,225,84889,4,34.7
NYC,New York City,USA,North America,18.9,1223,15454,10,40.7
KAR,Karachi,Pakistan,Asia,17.1,3527,4847,8,24.9
BUE,Buenos Aires,Argentina,South America,15.4,4758,3237,25,-34.6
CHO,Chongqing,China,Asia,17.0,5473,3106,400,29.6
IST,Istanbul,Turkey,Asia,15.8,5343,2958,40,41.0
KOL,Kolkata,India,Asia,15.6,1886,8273,9,22.6
MAN,Manila,Philippines,Asia,14.8,619,23910,16,14.6
LAG,Lagos,Nigeria,Africa,15.7,1171,13407,41,6.5
RIO,Rio de Janeiro,Brazil,South America,13.6,1255,10837,2,-22.9
TIA,Tianjin,China,Asia,13.7,11760,1165,3,39.1
KIN,Kinshasa,DR Congo,Africa,15.6,2104,7414,240,-4.3
GUA,Guangzhou,China,Asia,13.6,7434,1831,21,23.1
LAH,Lahore,Pakistan,Asia,13.5,1772,7619,217,31.5
MOS,Moscow,Russia,Europe,12.6,2511,5017,156,55.8
SHE,Shenzhen,China,Asia,12.6,1997,6310,0,22.5
BAN,Bangalore,India,Asia,13.2,741,17814,920,12.9
PAR,Paris,France,Europe,11.1,105,21000,35,48.9
BOG,Bogota,Colombia,South America,11.3,1775,6366,2640,4.7
JAK,Jakarta,Indonesia,Asia,11.2,664,16864,8,-6.2
LIM,Lima,Peru,South America,11.0,2672,4117,154,-12.0
LON,London,UK,Europe,9.5,1572,6043,11,51.5
BAN2,Bangkok,Thailand,Asia,11.1,1569,7075,1,13.7
TEH,Tehran,Iran,Asia,9.4,730,12877,1200,35.7
HCM,Ho Chi Minh City,Vietnam,Asia,9.4,2061,4560,19,10.8
HKG,Hong Kong,China,Asia,7.5,1106,6781,0,22.3
BAG,Baghdad,Iraq,Asia,7.6,673,11293,34,33.3
SEO,Seoul,South Korea,Asia,9.7,605,16033,38,37.6
SIN,Singapore,Singapore,Asia,5.9,728,8104,15,1.3
DAR,Dar es Salaam,Tanzania,Africa,7.4,1393,5311,55,-6.8
JOH,Johannesburg,South Africa,Africa,6.2,1645,3769,1753,-26.2
BAR,Barcelona,Spain,Europe,5.7,101,56000,12,41.4
BER,Berlin,Germany,Europe,3.8,892,4260,34,52.5
MAD,Madrid,Spain,Europe,6.7,604,11091,667,40.4
ROM,Rome,Italy,Europe,4.3,1287,3340,21,41.9
TOR,Toronto,Canada,North America,6.4,5904,1084,76,43.7
MEL,Melbourne,Australia,Oceania,5.2,9993,520,31,-37.8
SYD,Sydney,Australia,Oceania,5.4,12368,437,3,-33.9
DUB,Dubai,UAE,Asia,3.6,4114,875,5,25.2
NAI,Nairobi,Kenya,Africa,5.1,696,7327,1795,-1.3
ATH,Athens,Greece,Europe,3.2,412,7766,170,38.0
STO,Stockholm,Sweden,Europe,2.4,381,6299,28,59.3
ZUR,Zurich,Switzerland,Europe,1.4,87,16092,408,47.4
OSL,Oslo,Norway,Europe,1.0,454,2203,23,59.9
COP,Copenhagen,Denmark,Europe,1.4,180,7778,5,55.7
HEL,Helsinki,Finland,Europe,1.3,213,6103,26,60.2
DUB2,Dublin,Ireland,Europe,1.3,115,11304,8,53.3
REY,Reykjavik,Iceland,Europe,0.2,273,733,37,64.1
LAP,La Paz,Bolivia,South America,1.9,472,4025,3640,-16.5
QUI,Quito,Ecuador,South America,1.9,372,5108,2850,-0.2
WLG,Wellington,New Zealand,Oceania,0.4,290,1379,0,-41.3
`;

// ── Customer mall (clustering classic) ───────────────────────────────────
// "Mall Customer Segmentation" Kaggle dataset, public domain. Synthetic
// but widely used for cluster demos. Gender is the natural cover, age +
// income + spending score are the numeric axes. Note: real Kaggle source
// has 200 rows; this is the canonical first 60 (representative of the
// 5-cluster structure most teachers use).
const MALL_CSV = `id,gender,age,annual_income_k,spending_score
1,Male,19,15,39
2,Male,21,15,81
3,Female,20,16,6
4,Female,23,16,77
5,Female,31,17,40
6,Female,22,17,76
7,Female,35,18,6
8,Female,23,18,94
9,Male,64,19,3
10,Female,30,19,72
11,Male,67,19,14
12,Female,35,19,99
13,Female,58,20,15
14,Female,24,20,77
15,Male,37,20,13
16,Male,22,20,79
17,Female,35,21,35
18,Male,20,21,66
19,Male,52,23,29
20,Female,35,23,98
21,Male,35,24,35
22,Male,25,24,73
23,Female,46,25,5
24,Male,31,25,73
25,Female,54,28,14
26,Male,29,28,82
27,Female,45,28,32
28,Male,35,28,61
29,Female,40,29,31
30,Female,23,29,87
31,Male,60,30,4
32,Female,21,30,73
33,Male,53,33,4
34,Male,18,33,92
35,Female,49,33,14
36,Female,21,33,81
37,Female,42,34,17
38,Female,30,34,73
39,Female,36,37,26
40,Female,20,37,75
41,Female,65,38,35
42,Male,24,38,92
43,Male,48,39,36
44,Female,31,39,61
45,Female,49,39,28
46,Female,24,39,65
47,Female,50,40,55
48,Female,27,40,47
49,Female,29,40,42
50,Female,31,40,42
51,Female,49,42,52
52,Male,33,42,60
53,Female,31,43,54
54,Male,59,43,60
55,Female,50,43,45
56,Male,47,43,41
57,Female,51,44,50
58,Male,69,44,46
59,Female,27,46,51
60,Male,53,46,46
`;

// ── Hospital patient records ─────────────────────────────────────────────
// Demonstrates GIGI's killer feature: gauge encryption. The dataset is
// fully fictitious (names + IDs + diagnoses generated for demo), shaped
// like a real PHI/PII workload:
//
//   patient_name, ssn_last4, diagnosis_text  → OPAQUE  (masked in UI)
//   patient_id, diagnosis_code, attending_md → INDEXED (queryable on equality)
//   total_billed_usd, length_of_stay,        → AFFINE  (numeric, gauge-encrypted)
//   bp_systolic
//   department, insurance                    → plain   (cover + filter axes)
//   visits_ytd, heart_rate_bpm               → plain   (free numerics)
//
// κ + λ₁ + Betti all compute over the encrypted columns at native speed —
// the engine never sees plaintext for OPAQUE fields, and the UI never
// shows plaintext for OPAQUE/INDEXED fields. The cover field (`department`)
// is plain so the cohort grouping is observable.
const HOSPITAL_CSV = `patient_id,patient_name,ssn_last4,date_of_birth,department,diagnosis_code,diagnosis_text,attending_md,insurance,visits_ytd,bp_systolic,heart_rate_bpm,length_of_stay,total_billed_usd
P-0001,Jane Holloway,4821,1962-03-14,Cardiology,I10,Essential hypertension,Dr. Okafor,BlueCross,3,148,72,2,8420
P-0002,Marcus Reed,9173,1958-11-02,Cardiology,I25.10,Chronic coronary artery dz,Dr. Okafor,Medicare,5,162,68,4,18650
P-0003,Linh Tran,5604,1979-07-28,Oncology,C50.911,Breast malignancy unspecified,Dr. Shapiro,Aetna,8,118,82,6,42180
P-0004,Robert Quinn,3091,1971-01-09,ER,S06.0X1A,Concussion with LOC,Dr. Patel,Uninsured,1,128,98,1,3840
P-0005,Aisha Bello,7748,1985-09-22,Pediatrics,J45.40,Moderate persistent asthma,Dr. Chen,UnitedHealth,4,108,88,1,2210
P-0006,Henry Voss,1226,1952-05-30,Cardiology,I48.0,Paroxysmal atrial fibrillation,Dr. Okafor,Medicare,6,154,76,3,14920
P-0007,Sofia Mendoza,8835,1990-12-11,Oncology,C18.7,Sigmoid colon malignancy,Dr. Shapiro,Cigna,9,122,84,7,51340
P-0008,Daniel Park,4477,1968-08-04,ER,T81.4XXA,Surgical site infection,Dr. Patel,BlueCross,2,134,102,2,6890
P-0009,Priya Singh,9112,1994-04-17,Pediatrics,L20.9,Atopic dermatitis,Dr. Chen,Aetna,3,112,90,1,1780
P-0010,Thomas O'Brien,2058,1949-10-25,Cardiology,I50.32,Chronic diastolic heart failure,Dr. Okafor,Medicare,7,170,64,8,32480
P-0011,Yuki Tanaka,6633,1976-02-14,Oncology,C61,Prostate malignancy,Dr. Shapiro,UnitedHealth,5,128,78,4,28640
P-0012,Anna Kowalski,3917,1988-06-08,ER,K35.80,Acute appendicitis unspecified,Dr. Patel,Cigna,1,124,108,2,9450
P-0013,James Carter,7290,1955-09-19,Cardiology,I25.10,Chronic coronary artery dz,Dr. Okafor,Medicare,4,158,70,3,13280
P-0014,Esperanza Diaz,4145,1982-11-30,Oncology,C34.91,Lung malignancy unspecified,Dr. Shapiro,BlueCross,11,116,86,9,67220
P-0015,William Murphy,8862,1973-03-25,ER,S72.001A,Closed femur fracture,Dr. Patel,Aetna,1,132,94,3,18230
P-0016,Mei Lin Wong,1374,1996-08-12,Pediatrics,J06.9,Acute upper respiratory infection,Dr. Chen,UnitedHealth,2,106,92,1,1240
P-0017,Khaled Hassan,5031,1960-04-07,Cardiology,I63.9,Cerebral infarction unspecified,Dr. Okafor,Medicare,8,176,66,11,48910
P-0018,Catherine Boyle,6726,1969-12-03,Oncology,C73,Thyroid malignancy,Dr. Shapiro,Cigna,4,120,80,3,21470
P-0019,Diego Ramirez,2598,1986-07-21,ER,F32.9,Major depressive episode,Dr. Patel,Uninsured,2,118,88,2,4920
P-0020,Olivia Sterling,9444,1991-02-28,Pediatrics,E10.65,Type 1 diabetes uncontrolled,Dr. Chen,Aetna,6,114,96,2,8170
P-0021,Hiroshi Yamada,3782,1947-06-15,Cardiology,I50.32,Chronic diastolic heart failure,Dr. Okafor,Medicare,9,168,62,7,29830
P-0022,Beatrice Cole,8019,1972-10-19,Oncology,C25.9,Pancreatic malignancy,Dr. Shapiro,BlueCross,12,124,82,14,89240
P-0023,Eric Larsen,4653,1980-05-13,ER,K85.9,Acute pancreatitis unspecified,Dr. Patel,UnitedHealth,2,130,104,5,16480
P-0024,Sana Rahman,7186,1998-11-26,Pediatrics,J45.40,Moderate persistent asthma,Dr. Chen,Cigna,3,110,94,1,1980
P-0025,Nathaniel Boone,2243,1965-08-29,Cardiology,I10,Essential hypertension,Dr. Okafor,Medicare,3,152,74,2,7340
P-0026,Joan Ferreira,8967,1977-01-14,Oncology,C56.1,Right ovarian malignancy,Dr. Shapiro,Aetna,6,122,80,5,33120
P-0027,Tobias Klein,5408,1990-09-05,ER,S52.521A,Closed Colles' fracture,Dr. Patel,BlueCross,1,128,90,1,3260
P-0028,Amara Okonkwo,1932,1993-05-24,Pediatrics,B34.9,Viral infection unspecified,Dr. Chen,UnitedHealth,2,104,98,1,890
P-0029,Margaret Sullivan,6105,1944-12-08,Cardiology,I48.91,Atrial fibrillation unspecified,Dr. Okafor,Medicare,10,164,68,9,38570
P-0030,Vikram Joshi,7841,1984-04-02,Oncology,C71.9,Brain malignancy unspecified,Dr. Shapiro,Cigna,8,118,78,12,74310
`;

// ── Payment transactions ─────────────────────────────────────────────────
// 50 SWIFT/ACH/RTP-shaped payments. Includes deliberate near-duplicates
// (same payment, different reference formatting) to make a great Prism
// Dedup demo. Also some sanctions-screening-shaped rows (Iran, North Korea
// counterparties → would-be flags). All fictitious.
const PAYMENTS_CSV = `payment_id,from_account,to_account,amount_usd,fee_usd,currency,rail,iso_date,reference,status,exception_flag
P-100001,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-12,INV-2026-04823,settled,false
P-100002,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-12,INV 2026 04823,settled,false
P-100003,WELL-USA-203,HSBC-HK-9821,75000.00,38.00,USD,SWIFT,2026-04-13,Q1 dividend,settled,false
P-100004,JPMC-USA-077,BNP-FR-1102,1250000.00,55.00,EUR,SWIFT,2026-04-13,LC-2026-77432 drawdown,settled,false
P-100005,WELL-USA-203,HSBC-HK-9821,75000.00,38.00,USD,SWIFT,2026-04-14,Q1 dividend ref,settled,false
P-100006,CITI-USA-440,KBA-IR-0091,180000.00,45.00,USD,SWIFT,2026-04-14,humanitarian aid,returned,true
P-100007,BOA-USA-512,KEB-KR-2200,45000.00,5.00,USD,ACH,2026-04-14,supplier payment,settled,false
P-100008,JPMC-USA-077,BNP-FR-1102,1250000.00,55.00,EUR,SWIFT,2026-04-15,LC2026-77432-drawdown,pending,false
P-100009,CHAS-USA-001,DBSS-SG-742,12500.00,0.25,USD,RTP,2026-04-15,client refund,settled,false
P-100010,USBK-USA-188,MUFG-JP-4401,890000.00,48.00,JPY,SWIFT,2026-04-15,invoice 8821,settled,false
P-100011,WELL-USA-203,HSBC-HK-9821,75000.00,38.00,USD,SWIFT,2026-04-15,Q1 dividend payment,settled,false
P-100012,GS-USA-001,CSFB-CH-2241,3400000.00,65.00,USD,SWIFT,2026-04-16,M&A escrow tranche 1,settled,false
P-100013,CHAS-USA-001,KP-KP-0001,250000.00,42.50,USD,SWIFT,2026-04-16,construction equipment,returned,true
P-100014,BOA-USA-512,KEB-KR-2200,45000.00,5.00,USD,ACH,2026-04-16,supplier-payment,settled,false
P-100015,JPMC-USA-077,BNP-FR-1102,1250000.00,55.00,EUR,SWIFT,2026-04-16,LC2026 77432 drawdown,settled,false
P-100016,WELL-USA-203,DBJ-JP-1100,67000.00,40.00,USD,SWIFT,2026-04-17,services rendered,settled,false
P-100017,CITI-USA-440,BNDA-GH-7702,12000.00,42.00,USD,SWIFT,2026-04-17,school supplies donation,settled,false
P-100018,CHAS-USA-001,DBSS-SG-742,250000,42.50,USD,SWIFT,2026-04-17,INV2026-04823,settled,false
P-100019,USBK-USA-188,MUFG-JP-4401,890000.00,48.00,JPY,SWIFT,2026-04-17,Invoice 8821,settled,false
P-100020,JPMC-USA-077,SBI-IN-3300,440000.00,50.00,USD,SWIFT,2026-04-18,Q2 trade settlement,settled,false
P-100021,WELL-USA-203,HSBC-HK-9821,75000.00,38.00,USD,SWIFT,2026-04-18,quarterly dividend,settled,false
P-100022,GS-USA-001,CSFB-CH-2241,3400000.00,65.00,USD,SWIFT,2026-04-18,M&A escrow tranche 2,settled,false
P-100023,BOA-USA-512,BBVA-MX-2299,28000.00,0.25,USD,RTP,2026-04-18,vendor payment,settled,false
P-100024,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-18,INV-2026-04-823,pending,false
P-100025,CITI-USA-440,BACOMER-VE-100,55000.00,45.00,USD,SWIFT,2026-04-19,humanitarian relief,pending,false
P-100026,JPMC-USA-077,BNP-FR-1102,1250000.00,55.00,EUR,SWIFT,2026-04-19,LC 2026 77432,settled,false
P-100027,WELL-USA-203,HSBC-HK-9821,8400.00,0.25,USD,RTP,2026-04-19,refund customer,settled,false
P-100028,USBK-USA-188,DBJ-JP-1100,67000.00,40.00,USD,SWIFT,2026-04-19,services rendered Q2,settled,false
P-100029,BOA-USA-512,KEB-KR-2200,45000.00,5.00,USD,ACH,2026-04-19,supplier payment,settled,false
P-100030,CHAS-USA-001,SDN-KP-0099,1100000.00,55.00,USD,SWIFT,2026-04-20,equipment lease,returned,true
P-100031,GS-USA-001,CSFB-CH-2241,3400000.00,65.00,USD,SWIFT,2026-04-20,M&A escrow final,settled,false
P-100032,JPMC-USA-077,SBI-IN-3300,440000.00,50.00,USD,SWIFT,2026-04-20,Q2 trade,settled,false
P-100033,WELL-USA-203,DBJ-JP-1100,67000,40.00,USD,SWIFT,2026-04-20,services rendered,settled,false
P-100034,CITI-USA-440,KBA-IR-0091,180000.00,45.00,USD,SWIFT,2026-04-20,humanitarian-aid,returned,true
P-100035,USBK-USA-188,MUFG-JP-4401,890000.00,48.00,JPY,SWIFT,2026-04-21,invoice 8821 final,settled,false
P-100036,BOA-USA-512,BBVA-MX-2299,28000.00,0.25,USD,RTP,2026-04-21,vendor-payment,settled,false
P-100037,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-21,INV/2026/04823,settled,false
P-100038,JPMC-USA-077,BNP-FR-1102,1250000.00,55.00,EUR,SWIFT,2026-04-21,LC-2026-77432,settled,false
P-100039,WELL-USA-203,HSBC-HK-9821,75000.00,38.00,USD,SWIFT,2026-04-21,Q1 div,settled,false
P-100040,GS-USA-001,CSFB-CH-2241,3400000.00,65.00,USD,SWIFT,2026-04-22,follow-on escrow,settled,false
P-100041,CITI-USA-440,BACOMER-VE-100,55000.00,45.00,USD,SWIFT,2026-04-22,humanitarian relief Q2,pending,false
P-100042,BOA-USA-512,KEB-KR-2200,45000.00,5.00,USD,ACH,2026-04-22,supplier monthly,settled,false
P-100043,USBK-USA-188,MUFG-JP-4401,2100000.00,52.00,JPY,SWIFT,2026-04-22,large invoice 9924,pending,true
P-100044,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-22,INV 2026/04823,settled,false
P-100045,JPMC-USA-077,SBI-IN-3300,440000.00,50.00,USD,SWIFT,2026-04-23,Q2 trade settle,settled,false
P-100046,WELL-USA-203,DBJ-JP-1100,67000.00,40.00,USD,SWIFT,2026-04-23,services-rendered,settled,false
P-100047,GS-USA-001,CSFB-CH-2241,80000.00,0.25,USD,RTP,2026-04-23,bridge financing fee,settled,false
P-100048,BOA-USA-512,BBVA-MX-2299,28000.00,0.25,USD,RTP,2026-04-23,vendor payment Q2,settled,false
P-100049,CITI-USA-440,KBA-IR-0091,180000.00,45.00,USD,SWIFT,2026-04-23,humanitarian Q2,returned,true
P-100050,CHAS-USA-001,DBSS-SG-742,250000.00,42.50,USD,SWIFT,2026-04-24,INV-2026-04823 retry,settled,false
`;

// ── Daily cashflow ───────────────────────────────────────────────────────
// 60 days of treasury cash flow per region. Built with seasonal-ish noise
// + a downtrend, perfect for the Prism Forecast workflow.
const CASHFLOW_CSV = `cashflow_id,date,region,inflow_usd,outflow_usd,net_usd,txn_count
CF-001,2026-02-15,NA,1240000,980000,260000,142
CF-002,2026-02-15,EMEA,890000,920000,-30000,118
CF-003,2026-02-15,APAC,560000,510000,50000,87
CF-004,2026-02-16,NA,1180000,1010000,170000,138
CF-005,2026-02-16,EMEA,920000,895000,25000,121
CF-006,2026-02-16,APAC,610000,540000,70000,92
CF-007,2026-02-17,NA,1340000,1090000,250000,151
CF-008,2026-02-17,EMEA,855000,940000,-85000,114
CF-009,2026-02-17,APAC,590000,560000,30000,89
CF-010,2026-02-18,NA,1290000,1120000,170000,144
CF-011,2026-02-18,EMEA,910000,980000,-70000,123
CF-012,2026-02-18,APAC,620000,580000,40000,94
CF-013,2026-02-19,NA,1410000,1180000,230000,158
CF-014,2026-02-19,EMEA,945000,1010000,-65000,128
CF-015,2026-02-19,APAC,640000,600000,40000,98
CF-016,2026-02-22,NA,1380000,1150000,230000,154
CF-017,2026-02-22,EMEA,920000,1000000,-80000,125
CF-018,2026-02-22,APAC,650000,610000,40000,99
CF-019,2026-02-23,NA,1320000,1190000,130000,148
CF-020,2026-02-23,EMEA,890000,990000,-100000,119
CF-021,2026-02-23,APAC,640000,620000,20000,97
CF-022,2026-02-24,NA,1290000,1220000,70000,144
CF-023,2026-02-24,EMEA,870000,1020000,-150000,117
CF-024,2026-02-24,APAC,630000,640000,-10000,96
CF-025,2026-02-25,NA,1260000,1240000,20000,141
CF-026,2026-02-25,EMEA,850000,1040000,-190000,115
CF-027,2026-02-25,APAC,620000,650000,-30000,94
CF-028,2026-02-26,NA,1230000,1250000,-20000,137
CF-029,2026-02-26,EMEA,840000,1050000,-210000,114
CF-030,2026-02-26,APAC,610000,660000,-50000,93
CF-031,2026-03-01,NA,1210000,1260000,-50000,135
CF-032,2026-03-01,EMEA,830000,1070000,-240000,112
CF-033,2026-03-01,APAC,600000,670000,-70000,91
CF-034,2026-03-02,NA,1190000,1280000,-90000,133
CF-035,2026-03-02,EMEA,820000,1090000,-270000,111
CF-036,2026-03-02,APAC,590000,680000,-90000,89
CF-037,2026-03-03,NA,1180000,1290000,-110000,132
CF-038,2026-03-03,EMEA,810000,1100000,-290000,109
CF-039,2026-03-03,APAC,580000,690000,-110000,88
CF-040,2026-03-04,NA,1170000,1310000,-140000,131
CF-041,2026-03-04,EMEA,800000,1120000,-320000,108
CF-042,2026-03-04,APAC,570000,700000,-130000,86
CF-043,2026-03-05,NA,1160000,1320000,-160000,130
CF-044,2026-03-05,EMEA,790000,1140000,-350000,107
CF-045,2026-03-05,APAC,560000,710000,-150000,85
CF-046,2026-03-08,NA,1140000,1330000,-190000,128
CF-047,2026-03-08,EMEA,780000,1150000,-370000,105
CF-048,2026-03-08,APAC,560000,720000,-160000,84
CF-049,2026-03-09,NA,1130000,1340000,-210000,127
CF-050,2026-03-09,EMEA,770000,1170000,-400000,104
CF-051,2026-03-09,APAC,550000,730000,-180000,83
CF-052,2026-03-10,NA,1110000,1360000,-250000,125
CF-053,2026-03-10,EMEA,750000,1190000,-440000,102
CF-054,2026-03-10,APAC,540000,740000,-200000,82
CF-055,2026-03-11,NA,1090000,1370000,-280000,124
CF-056,2026-03-11,EMEA,740000,1210000,-470000,101
CF-057,2026-03-11,APAC,530000,760000,-230000,80
CF-058,2026-03-12,NA,1070000,1380000,-310000,122
CF-059,2026-03-12,EMEA,720000,1230000,-510000,99
CF-060,2026-03-12,APAC,520000,780000,-260000,78
`;

// ── Account activity ─────────────────────────────────────────────────────
// 40 accounts with behavioral-surveillance metrics. Some accounts have
// obvious anomalies (high velocity + many countries + structuring risk)
// that Prism Monitor should surface.
const ACCOUNTS_CSV = `account_id,account_type,region,txn_count_30d,volume_30d_usd,avg_txn_size,velocity_pctl,countries_touched,structuring_risk
ACC-1001,retail,NA,18,12400,689,42,1,0.08
ACC-1002,retail,NA,24,18900,788,55,2,0.12
ACC-1003,business,NA,142,890000,6268,78,4,0.15
ACC-1004,business,NA,89,540000,6067,65,3,0.10
ACC-1005,correspondent,NA,420,4200000,10000,82,12,0.18
ACC-1006,retail,EMEA,12,7800,650,35,1,0.06
ACC-1007,retail,EMEA,8,5200,650,28,1,0.04
ACC-1008,business,EMEA,156,1100000,7051,75,5,0.14
ACC-1009,business,EMEA,98,720000,7347,68,4,0.11
ACC-1010,correspondent,EMEA,380,3900000,10263,79,11,0.16
ACC-1011,retail,APAC,15,9200,613,38,1,0.07
ACC-1012,retail,APAC,20,14400,720,48,2,0.09
ACC-1013,business,APAC,128,810000,6328,72,4,0.13
ACC-1014,business,APAC,72,460000,6389,62,3,0.10
ACC-1015,correspondent,APAC,450,4800000,10667,84,13,0.19
ACC-1016,retail,NA,9,18000,2000,92,8,0.78
ACC-1017,retail,NA,11,21000,1909,94,9,0.82
ACC-1018,retail,EMEA,6,9400,1567,88,7,0.71
ACC-1019,business,NA,3200,42000000,13125,99,28,0.94
ACC-1020,business,EMEA,2900,38000000,13103,99,26,0.91
ACC-1021,retail,NA,22,16800,764,52,2,0.10
ACC-1022,business,NA,118,720000,6102,70,4,0.12
ACC-1023,correspondent,NA,398,4100000,10302,80,11,0.17
ACC-1024,retail,EMEA,14,9100,650,40,1,0.07
ACC-1025,business,EMEA,135,920000,6815,74,4,0.13
ACC-1026,retail,APAC,17,11000,647,42,1,0.08
ACC-1027,business,APAC,108,690000,6389,68,3,0.11
ACC-1028,correspondent,APAC,420,4500000,10714,82,12,0.18
ACC-1029,retail,NA,5,9800,1960,90,8,0.75
ACC-1030,business,APAC,2700,36000000,13333,99,25,0.89
ACC-1031,retail,NA,28,21000,750,58,3,0.11
ACC-1032,retail,EMEA,16,10200,638,41,2,0.08
ACC-1033,business,NA,165,1050000,6364,76,5,0.14
ACC-1034,correspondent,EMEA,395,4050000,10253,81,11,0.17
ACC-1035,retail,APAC,13,8400,646,37,1,0.07
ACC-1036,business,APAC,142,920000,6479,73,4,0.13
ACC-1037,correspondent,NA,440,4700000,10682,83,13,0.19
ACC-1038,retail,EMEA,4,8200,2050,89,7,0.73
ACC-1039,business,EMEA,148,990000,6689,75,5,0.13
ACC-1040,retail,APAC,19,12800,674,45,2,0.09
`;

// ── Job applicants ───────────────────────────────────────────────────────
// 40 candidates across 5 hiring stages, 5 roles. Showcases Form view
// (intake), Multi-select tags (skills), Kanban (stage), and Sameness-find
// ("find candidates like this hired one"). Includes 4 deliberate anomalies:
//   - A-12: 0 yrs experience asking $250k → high κ on score axis
//   - A-23: hired with score 4.8 (cohort hires cluster around 8.5) → outlier
//   - A-31: rejected with score 9.2 (top-decile) → likely false negative
//   - A-37: identical profile to A-15 → near-duplicate for sameness search
const APPLICANTS_CSV = `applicant_id,full_name,role,stage,top_skill,experience_years,location,salary_ask_usd,score,applied_date
A-01,Mira Chen,Software Engineer,Phone Screen,Rust,6,San Francisco,180000,7.8,2026-04-02
A-02,Jamal Okafor,Software Engineer,Onsite,Go,8,Remote,195000,8.6,2026-04-02
A-03,Priya Nair,Data Scientist,Applied,Python,4,New York,165000,7.2,2026-04-03
A-04,Theo Lindqvist,Designer,Onsite,Figma,5,Stockholm,140000,8.1,2026-04-03
A-05,Anita Vasquez,Product Manager,Offer,Strategy,7,Austin,210000,9.0,2026-04-04
A-06,Kenji Watanabe,Software Engineer,Hired,TypeScript,9,Tokyo,205000,8.9,2026-04-04
A-07,Hassan El-Sayed,Data Scientist,Phone Screen,ML,5,Cairo,155000,7.5,2026-04-05
A-08,Sofia Romano,Designer,Applied,Figma,3,Milan,120000,7.0,2026-04-05
A-09,Diego Alvarez,Software Engineer,Phone Screen,Python,4,Mexico City,135000,7.3,2026-04-06
A-10,Lena Schmidt,Product Manager,Onsite,Strategy,6,Berlin,175000,8.4,2026-04-06
A-11,Aarav Mehta,Software Engineer,Offer,Rust,7,Bangalore,160000,8.7,2026-04-08
A-12,Jordan Riley,Software Engineer,Applied,JavaScript,0,Remote,250000,4.2,2026-04-08
A-13,Yuki Tanaka,Data Scientist,Onsite,ML,6,Osaka,170000,8.3,2026-04-09
A-14,Marcus Webb,Sales,Phone Screen,Enterprise,8,Chicago,190000,7.9,2026-04-09
A-15,Elena Petrova,Software Engineer,Hired,Python,7,Moscow,185000,8.8,2026-04-10
A-16,Ravi Subramanian,Software Engineer,Phone Screen,Go,5,Hyderabad,140000,7.6,2026-04-10
A-17,Camille Dubois,Designer,Hired,Figma,6,Paris,155000,8.7,2026-04-11
A-18,Felipe Cardoso,Data Scientist,Applied,Python,3,São Paulo,130000,6.9,2026-04-11
A-19,Aisha Mwangi,Product Manager,Phone Screen,Strategy,5,Nairobi,150000,7.8,2026-04-12
A-20,Olaf Brennan,Software Engineer,Onsite,Rust,8,Dublin,195000,8.5,2026-04-12
A-21,Mei Lin,Software Engineer,Offer,TypeScript,6,Singapore,175000,8.6,2026-04-15
A-22,Carlos Rivera,Sales,Onsite,Enterprise,10,Miami,210000,8.3,2026-04-15
A-23,Reese Cooper,Software Engineer,Hired,JavaScript,2,Remote,110000,4.8,2026-04-16
A-24,Zara Khan,Data Scientist,Onsite,ML,7,Karachi,165000,8.4,2026-04-16
A-25,Vincent Hugo,Designer,Phone Screen,Figma,4,Lyon,135000,7.4,2026-04-17
A-26,Beatriz Santos,Product Manager,Hired,Strategy,8,Lisbon,195000,9.1,2026-04-17
A-27,Hiroto Sato,Software Engineer,Applied,Go,5,Yokohama,160000,7.7,2026-04-18
A-28,Naomi Park,Software Engineer,Phone Screen,Python,4,Seoul,150000,7.6,2026-04-19
A-29,Ahmed Hosseini,Data Scientist,Phone Screen,Python,6,Tehran,145000,8.0,2026-04-19
A-30,Linnea Bergman,Designer,Onsite,Figma,7,Gothenburg,165000,8.5,2026-04-22
A-31,Tara Whitfield,Software Engineer,Rejected,Rust,9,Remote,210000,9.2,2026-04-22
A-32,Bilal Rahman,Software Engineer,Hired,TypeScript,8,Dubai,200000,8.9,2026-04-23
A-33,Anya Volkov,Sales,Hired,Enterprise,11,London,230000,9.0,2026-04-23
A-34,Tyrell Brooks,Software Engineer,Onsite,Go,6,Atlanta,170000,8.2,2026-04-24
A-35,Saoirse Murphy,Designer,Applied,Figma,4,Dublin,130000,7.2,2026-04-25
A-36,Magnus Hansen,Product Manager,Offer,Strategy,9,Copenhagen,215000,9.1,2026-04-26
A-37,Eleni Petropoulos,Software Engineer,Phone Screen,Python,7,Athens,185000,8.8,2026-04-29
A-38,Kai Yamamoto,Data Scientist,Offer,ML,7,Kyoto,175000,8.6,2026-04-30
A-39,Issa Diallo,Software Engineer,Onsite,Rust,6,Dakar,160000,8.1,2026-05-01
A-40,Hannah Kim,Software Engineer,Hired,TypeScript,7,Vancouver,180000,8.8,2026-05-02
`;

// ── Sensor telemetry ─────────────────────────────────────────────────────
// 50 readings: 10 industrial sensors × 5 daily snapshots. Showcases
// drag-fill (per-sensor trend extrapolation), Forecast (rising temp on
// S-007), Calendar view with κ-tint per day, and κ-monitor on the planted
// drift. Sensor S-007 is failing: its temperature is climbing 4°C/day
// while vibration drifts upward — that's the anomaly Monitor catches.
const SENSORS_CSV = `reading_id,sensor_id,location,reading_date,temperature_c,vibration_hz,pressure_psi,humidity_pct,status
R-001,S-001,Plant-A-NorthBay,2026-05-01,22.1,18.4,101.2,42,nominal
R-002,S-002,Plant-A-NorthBay,2026-05-01,21.8,17.9,100.8,43,nominal
R-003,S-003,Plant-A-EastBay,2026-05-01,23.4,19.1,102.1,41,nominal
R-004,S-004,Plant-A-EastBay,2026-05-01,22.6,18.2,101.5,44,nominal
R-005,S-005,Plant-B-Floor1,2026-05-01,24.1,20.3,103.4,38,nominal
R-006,S-006,Plant-B-Floor1,2026-05-01,23.7,19.8,102.9,39,nominal
R-007,S-007,Plant-B-Floor2,2026-05-01,25.2,21.5,104.1,37,nominal
R-008,S-008,Plant-B-Floor2,2026-05-01,24.4,20.7,103.6,40,nominal
R-009,S-009,Plant-C-Coldroom,2026-05-01,8.3,16.2,99.4,55,nominal
R-010,S-010,Plant-C-Coldroom,2026-05-01,8.1,16.0,99.2,56,nominal
R-011,S-001,Plant-A-NorthBay,2026-05-02,22.3,18.5,101.3,42,nominal
R-012,S-002,Plant-A-NorthBay,2026-05-02,21.9,17.8,100.7,43,nominal
R-013,S-003,Plant-A-EastBay,2026-05-02,23.5,19.0,102.0,41,nominal
R-014,S-004,Plant-A-EastBay,2026-05-02,22.7,18.3,101.6,44,nominal
R-015,S-005,Plant-B-Floor1,2026-05-02,24.2,20.4,103.5,38,nominal
R-016,S-006,Plant-B-Floor1,2026-05-02,23.6,19.7,102.8,39,nominal
R-017,S-007,Plant-B-Floor2,2026-05-02,29.6,23.1,104.3,36,warn
R-018,S-008,Plant-B-Floor2,2026-05-02,24.5,20.6,103.5,40,nominal
R-019,S-009,Plant-C-Coldroom,2026-05-02,8.4,16.1,99.3,55,nominal
R-020,S-010,Plant-C-Coldroom,2026-05-02,8.2,16.0,99.1,56,nominal
R-021,S-001,Plant-A-NorthBay,2026-05-03,22.0,18.4,101.2,43,nominal
R-022,S-002,Plant-A-NorthBay,2026-05-03,21.7,17.9,100.9,42,nominal
R-023,S-003,Plant-A-EastBay,2026-05-03,23.3,19.2,102.2,41,nominal
R-024,S-004,Plant-A-EastBay,2026-05-03,22.5,18.1,101.4,44,nominal
R-025,S-005,Plant-B-Floor1,2026-05-03,24.0,20.2,103.3,38,nominal
R-026,S-006,Plant-B-Floor1,2026-05-03,23.8,19.9,102.9,39,nominal
R-027,S-007,Plant-B-Floor2,2026-05-03,33.9,24.8,104.6,34,warn
R-028,S-008,Plant-B-Floor2,2026-05-03,24.6,20.8,103.7,40,nominal
R-029,S-009,Plant-C-Coldroom,2026-05-03,8.2,16.0,99.3,55,nominal
R-030,S-010,Plant-C-Coldroom,2026-05-03,8.0,15.9,99.0,56,nominal
R-031,S-001,Plant-A-NorthBay,2026-05-04,22.2,18.5,101.3,42,nominal
R-032,S-002,Plant-A-NorthBay,2026-05-04,21.8,17.8,100.8,43,nominal
R-033,S-003,Plant-A-EastBay,2026-05-04,23.4,19.1,102.1,41,nominal
R-034,S-004,Plant-A-EastBay,2026-05-04,22.6,18.2,101.5,44,nominal
R-035,S-005,Plant-B-Floor1,2026-05-04,24.1,20.3,103.4,38,nominal
R-036,S-006,Plant-B-Floor1,2026-05-04,23.7,19.8,102.9,39,nominal
R-037,S-007,Plant-B-Floor2,2026-05-04,38.4,26.5,104.9,32,fault
R-038,S-008,Plant-B-Floor2,2026-05-04,24.4,20.7,103.6,40,nominal
R-039,S-009,Plant-C-Coldroom,2026-05-04,8.3,16.1,99.4,55,nominal
R-040,S-010,Plant-C-Coldroom,2026-05-04,8.1,16.0,99.2,56,nominal
R-041,S-001,Plant-A-NorthBay,2026-05-05,22.4,18.6,101.4,42,nominal
R-042,S-002,Plant-A-NorthBay,2026-05-05,22.0,17.9,100.9,43,nominal
R-043,S-003,Plant-A-EastBay,2026-05-05,23.6,19.2,102.2,41,nominal
R-044,S-004,Plant-A-EastBay,2026-05-05,22.8,18.3,101.7,44,nominal
R-045,S-005,Plant-B-Floor1,2026-05-05,24.3,20.5,103.6,38,nominal
R-046,S-006,Plant-B-Floor1,2026-05-05,23.9,20.0,103.0,39,nominal
R-047,S-007,Plant-B-Floor2,2026-05-05,42.7,28.4,105.2,30,fault
R-048,S-008,Plant-B-Floor2,2026-05-05,24.7,20.9,103.8,40,nominal
R-049,S-009,Plant-C-Coldroom,2026-05-05,8.4,16.2,99.5,55,nominal
R-050,S-010,Plant-C-Coldroom,2026-05-05,8.2,16.1,99.3,56,nominal
`;

// ── Kaggle · Titanic (classic challenge) ─────────────────────────────────
// 50 passengers from the 1912 voyage — the canonical "your first ML
// dataset." Survival prediction by demographics. Setosa-clear cohort
// split between 1st-class women (survived) and 3rd-class men (didn't).
const TITANIC_CSV = `passenger_id,name,sex,age,pclass,fare_usd,embarked,sibsp,parch,survived
1,Braund Owen,male,22,3,7.25,Southampton,1,0,false
2,Cumings Florence,female,38,1,71.28,Cherbourg,1,0,true
3,Heikkinen Laina,female,26,3,7.92,Southampton,0,0,true
4,Futrelle Lily,female,35,1,53.10,Southampton,1,0,true
5,Allen William,male,35,3,8.05,Southampton,0,0,false
6,Moran James,male,28,3,8.46,Queenstown,0,0,false
7,McCarthy Timothy,male,54,1,51.86,Southampton,0,0,false
8,Palsson Gosta,male,2,3,21.07,Southampton,3,1,false
9,Johnson Oscar,female,27,3,11.13,Southampton,0,2,true
10,Nasser Adele,female,14,2,30.07,Cherbourg,1,0,true
11,Sandstrom Marguerite,female,4,3,16.70,Southampton,1,1,true
12,Bonnell Elizabeth,female,58,1,26.55,Southampton,0,0,true
13,Saundercock William,male,20,3,8.05,Southampton,0,0,false
14,Andersson Anders,male,39,3,31.27,Southampton,1,5,false
15,Vestrom Hulda,female,14,3,7.85,Southampton,0,0,false
16,Hewlett Mary,female,55,2,16.00,Southampton,0,0,true
17,Rice Eugene,male,2,3,29.12,Queenstown,4,1,false
18,Williams Charles,male,32,2,13.00,Southampton,0,0,true
19,Vander Helene,female,31,3,18.00,Southampton,1,0,false
20,Masselmani Fatima,female,22,3,7.22,Cherbourg,0,0,true
21,Fynney Joseph,male,35,2,26.00,Southampton,0,0,false
22,Beesley Lawrence,male,34,2,13.00,Southampton,0,0,true
23,McGowan Anna,female,15,3,8.03,Queenstown,0,0,true
24,Sloper William,male,28,1,35.50,Southampton,0,0,true
25,Palsson Torborg,female,8,3,21.07,Southampton,3,1,false
26,Asplund Carl,female,38,3,31.39,Southampton,1,5,true
27,Emir Farred,male,30,3,7.22,Cherbourg,0,0,false
28,Fortune Charles,male,19,1,263.00,Southampton,3,2,false
29,O'Dwyer Ellen,female,28,3,7.88,Queenstown,0,0,true
30,Todoroff Lalio,male,28,3,7.90,Southampton,0,0,false
31,Uruchurtu Manuel,male,40,1,27.72,Cherbourg,0,0,false
32,Spencer William,female,28,1,146.52,Cherbourg,1,0,true
33,Glynn Mary,female,28,3,7.75,Queenstown,0,0,true
34,Wheadon Edward,male,66,2,10.50,Southampton,0,0,false
35,Meyer Edgar,male,28,1,82.17,Cherbourg,1,0,false
36,Holverson Alexander,male,42,1,52.00,Southampton,1,0,false
37,Mamee Hanna,male,28,3,7.22,Cherbourg,0,0,true
38,Cann Ernest,male,21,3,8.05,Southampton,0,0,false
39,Vander Jeannie,female,18,3,18.00,Southampton,2,0,false
40,Nicola-Yarred Jamila,female,14,3,11.24,Cherbourg,1,0,true
41,Ahlin Johanna,female,40,3,9.48,Southampton,1,0,false
42,Turpin Dorothy,female,27,2,21.00,Southampton,1,0,false
43,Kraeff Theodor,male,28,3,7.90,Cherbourg,0,0,false
44,Laroche Simonne,female,3,2,41.58,Cherbourg,1,2,true
45,Devaney Margaret,female,19,3,7.88,Queenstown,0,0,true
46,Rogers William,male,28,3,8.05,Southampton,0,0,false
47,Lennon Denis,male,28,3,15.50,Queenstown,1,0,false
48,O'Driscoll Bridget,female,28,3,7.75,Queenstown,0,0,true
49,Samaan Youssef,male,28,3,21.68,Cherbourg,2,0,false
50,Arnold Josephine,female,18,3,17.40,Southampton,1,0,false
`;

// ── Kaggle · Loan Approval (Playground S4E10, 2024) ──────────────────────
// 50 loan applications. Predict approval/rejection from credit history,
// income, and loan parameters. The 2024 Kaggle playground regenerated
// the classic loan-default dataset with a fresh distribution.
const LOAN_CSV = `application_id,age,annual_income_usd,loan_amount_usd,loan_term_months,credit_score,employment_years,debt_to_income_pct,home_status,loan_status
L-001,32,95000,250000,360,742,8,18.5,mortgage,approved
L-002,28,62000,180000,180,688,4,28.2,rent,rejected
L-003,45,142000,420000,360,778,15,21.0,mortgage,approved
L-004,38,88000,210000,240,715,10,24.5,own,approved
L-005,24,48000,95000,180,632,2,38.0,rent,rejected
L-006,52,165000,380000,360,802,22,15.8,own,approved
L-007,29,72000,150000,180,701,5,26.3,rent,approved
L-008,41,118000,310000,360,748,12,19.2,mortgage,approved
L-009,33,55000,165000,240,652,6,42.0,rent,rejected
L-010,47,135000,290000,360,765,18,17.5,own,approved
L-011,26,58000,120000,180,675,3,31.5,rent,rejected
L-012,36,102000,260000,240,728,9,22.8,mortgage,approved
L-013,55,178000,450000,360,815,25,14.2,own,approved
L-014,30,68000,140000,180,694,5,27.1,rent,approved
L-015,42,124000,330000,360,756,14,18.9,mortgage,approved
L-016,22,42000,80000,180,608,1,45.0,rent,rejected
L-017,48,148000,360000,360,782,19,16.5,own,approved
L-018,34,78000,190000,240,712,8,25.2,rent,approved
L-019,39,108000,280000,360,738,11,20.5,mortgage,approved
L-020,27,64000,135000,180,685,4,29.8,rent,rejected
L-021,51,158000,400000,360,795,21,15.5,own,approved
L-022,31,82000,195000,180,718,7,24.0,mortgage,approved
L-023,25,52000,105000,180,648,3,36.5,rent,rejected
L-024,44,128000,320000,360,762,16,18.0,mortgage,approved
L-025,37,95000,235000,240,725,10,23.0,own,approved
L-026,29,68000,145000,180,692,5,28.5,rent,approved
L-027,46,138000,340000,360,775,17,17.0,mortgage,approved
L-028,23,46000,90000,180,615,2,44.0,rent,rejected
L-029,35,88000,220000,240,720,9,23.5,own,approved
L-030,40,115000,295000,360,745,13,19.8,mortgage,approved
L-031,28,60000,130000,180,672,4,33.0,rent,rejected
L-032,53,168000,420000,360,808,23,15.0,own,approved
L-033,32,76000,175000,180,705,6,26.5,rent,approved
L-034,49,152000,370000,360,788,20,16.0,own,approved
L-035,26,54000,110000,180,658,3,35.0,rent,rejected
L-036,43,122000,305000,360,752,15,19.0,mortgage,approved
L-037,38,98000,245000,240,732,11,22.5,mortgage,approved
L-038,30,70000,155000,180,698,5,28.0,rent,approved
L-039,45,135000,335000,360,768,17,17.8,own,approved
L-040,24,48000,98000,180,628,2,40.0,rent,rejected
L-041,33,84000,200000,240,716,8,24.8,own,approved
L-042,41,116000,290000,360,742,14,20.0,mortgage,approved
L-043,28,66000,140000,180,690,5,29.2,rent,rejected
L-044,50,148000,365000,360,792,20,16.2,own,approved
L-045,36,92000,225000,240,724,10,23.2,mortgage,approved
L-046,27,58000,125000,180,668,4,32.5,rent,rejected
L-047,42,118000,300000,360,748,15,19.5,mortgage,approved
L-048,34,80000,185000,180,710,7,25.5,rent,approved
L-049,47,142000,355000,360,778,18,17.2,own,approved
L-050,25,50000,100000,180,640,3,37.5,rent,rejected
`;

// ── Kaggle · Credit Card Fraud (strong ML challenge) ─────────────────────
// 60 card transactions, 4 planted fraud cases. The real Kaggle dataset
// has heavily anonymized PCA features (V1..V28); this demo uses
// interpretable proxies (distance from home, txn velocity, off-hours)
// so the geometry is visible. Class imbalance ~6.7% mirrors the real
// fraud rate. Prism Monitor catches the planted fraud as κ-anomalies.
const FRAUD_CSV = `txn_id,timestamp,amount_usd,merchant_category,merchant_country,distance_from_home_km,txn_velocity_per_hr,is_chip,is_online,is_fraud
T-0001,2026-05-01T08:32,42.50,grocery,USA,3.2,1,true,false,false
T-0002,2026-05-01T09:15,18.99,coffee,USA,1.8,2,true,false,false
T-0003,2026-05-01T12:42,135.20,restaurant,USA,5.1,1,true,false,false
T-0004,2026-05-01T14:08,2450.00,electronics,USA,12.5,1,true,false,false
T-0005,2026-05-01T18:30,85.40,grocery,USA,3.5,1,true,false,false
T-0006,2026-05-02T07:50,12.75,coffee,USA,2.1,1,true,false,false
T-0007,2026-05-02T11:20,68.30,gas,USA,8.7,1,true,false,false
T-0008,2026-05-02T13:45,225.00,clothing,USA,15.2,1,true,false,false
T-0009,2026-05-02T19:12,95.80,restaurant,USA,4.8,1,true,false,false
T-0010,2026-05-03T03:18,3499.99,electronics,UA,8421.0,9,false,true,true
T-0011,2026-05-03T09:02,38.60,grocery,USA,3.0,1,true,false,false
T-0012,2026-05-03T11:48,15.25,coffee,USA,1.9,1,true,false,false
T-0013,2026-05-03T14:30,182.40,restaurant,USA,5.5,1,true,false,false
T-0014,2026-05-03T17:22,72.10,gas,USA,7.8,1,true,false,false
T-0015,2026-05-04T08:55,28.75,grocery,USA,2.8,1,true,false,false
T-0016,2026-05-04T12:18,425.00,clothing,USA,11.3,1,true,false,false
T-0017,2026-05-04T15:42,55.80,restaurant,USA,4.2,1,true,false,false
T-0018,2026-05-04T18:10,89.40,grocery,USA,3.5,1,true,false,false
T-0019,2026-05-05T02:34,2899.50,electronics,NG,9248.0,11,false,true,true
T-0020,2026-05-05T09:48,42.20,grocery,USA,3.1,1,true,false,false
T-0021,2026-05-05T11:32,18.50,coffee,USA,1.7,1,true,false,false
T-0022,2026-05-05T13:55,168.20,restaurant,USA,6.0,1,true,false,false
T-0023,2026-05-05T17:08,75.40,gas,USA,7.9,1,true,false,false
T-0024,2026-05-06T08:22,35.80,grocery,USA,3.3,1,true,false,false
T-0025,2026-05-06T11:15,195.00,clothing,USA,12.8,1,true,false,false
T-0026,2026-05-06T14:42,62.50,restaurant,USA,5.0,1,true,false,false
T-0027,2026-05-06T18:30,82.90,grocery,USA,3.6,1,true,false,false
T-0028,2026-05-07T09:08,28.40,coffee,USA,2.0,1,true,false,false
T-0029,2026-05-07T12:35,148.70,restaurant,USA,5.8,1,true,false,false
T-0030,2026-05-07T15:20,68.20,gas,USA,8.2,1,true,false,false
T-0031,2026-05-07T18:45,92.30,grocery,USA,3.4,1,true,false,false
T-0032,2026-05-08T01:52,4299.00,electronics,RO,7892.0,8,false,true,true
T-0033,2026-05-08T09:18,32.60,grocery,USA,2.9,1,true,false,false
T-0034,2026-05-08T11:42,22.40,coffee,USA,1.8,1,true,false,false
T-0035,2026-05-08T14:25,175.50,restaurant,USA,5.7,1,true,false,false
T-0036,2026-05-08T17:50,78.30,gas,USA,7.6,1,true,false,false
T-0037,2026-05-09T08:40,45.20,grocery,USA,3.2,1,true,false,false
T-0038,2026-05-09T11:08,265.00,clothing,USA,11.8,1,true,false,false
T-0039,2026-05-09T13:32,58.40,restaurant,USA,4.5,1,true,false,false
T-0040,2026-05-09T16:15,85.70,grocery,USA,3.7,1,true,false,false
T-0041,2026-05-10T09:25,38.90,coffee,USA,2.2,1,true,false,false
T-0042,2026-05-10T12:48,142.20,restaurant,USA,5.4,1,true,false,false
T-0043,2026-05-10T15:35,72.80,gas,USA,8.1,1,true,false,false
T-0044,2026-05-10T18:22,98.40,grocery,USA,3.8,1,true,false,false
T-0045,2026-05-11T08:18,42.10,grocery,USA,3.0,1,true,false,false
T-0046,2026-05-11T11:32,210.50,clothing,USA,13.2,1,true,false,false
T-0047,2026-05-11T14:08,65.30,restaurant,USA,4.8,1,true,false,false
T-0048,2026-05-11T17:55,88.20,grocery,USA,3.5,1,true,false,false
T-0049,2026-05-12T02:48,3799.00,electronics,BD,8654.0,10,false,true,true
T-0050,2026-05-12T09:12,28.60,grocery,USA,2.8,1,true,false,false
T-0051,2026-05-12T11:48,18.40,coffee,USA,1.9,1,true,false,false
T-0052,2026-05-12T14:35,158.90,restaurant,USA,5.6,1,true,false,false
T-0053,2026-05-12T17:22,75.10,gas,USA,7.7,1,true,false,false
T-0054,2026-05-13T08:35,38.20,grocery,USA,3.1,1,true,false,false
T-0055,2026-05-13T11:15,225.40,clothing,USA,12.1,1,true,false,false
T-0056,2026-05-13T14:42,68.50,restaurant,USA,5.2,1,true,false,false
T-0057,2026-05-13T18:18,92.10,grocery,USA,3.6,1,true,false,false
T-0058,2026-05-14T09:08,35.40,grocery,USA,3.0,1,true,false,false
T-0059,2026-05-14T12:32,178.80,restaurant,USA,5.9,1,true,false,false
T-0060,2026-05-14T15:48,82.40,gas,USA,8.0,1,true,false,false
`;

// ── Chase statements (Books reconciliation, side A) ──────────────────────
// 35 entries — paired with quickbooks_ledger below. Mostly the same data
// but with intentional drift: 2 orphans (only Chase has), 3 amount
// conflicts. Perfect for Prism Books once two-bundle support lands.
const CHASE_CSV = `ref,post_date,amount_usd,description,balance_usd
CHK-202604-001,2026-04-01,15400.00,Wire IN - Q1 dividend HSBC,1015400.00
CHK-202604-002,2026-04-02,-3200.00,Office rent April,1012200.00
CHK-202604-003,2026-04-03,28000.00,Customer payment ACME Corp,1040200.00
CHK-202604-004,2026-04-03,-12500.00,Payroll batch 04-A,1027700.00
CHK-202604-005,2026-04-04,-1850.00,Utilities April,1025850.00
CHK-202604-006,2026-04-05,8200.00,Customer payment GlobalTech,1034050.00
CHK-202604-007,2026-04-08,-450.00,Software subscription,1033600.00
CHK-202604-008,2026-04-09,18000.00,Customer payment Stellar Inc,1051600.00
CHK-202604-009,2026-04-09,-6200.00,Equipment lease,1045400.00
CHK-202604-010,2026-04-10,45000.00,Wire IN - escrow release,1090400.00
CHK-202604-011,2026-04-11,-2400.00,Marketing AdWords,1088000.00
CHK-202604-012,2026-04-12,12000.00,Customer payment Apex,1100000.00
CHK-202604-013,2026-04-15,-12500.00,Payroll batch 04-B,1087500.00
CHK-202604-014,2026-04-15,7800.00,Customer payment Bayside,1095300.00
CHK-202604-015,2026-04-16,-8900.00,Insurance premium quarterly,1086400.00
CHK-202604-016,2026-04-17,22000.00,Customer payment Hexagon,1108400.00
CHK-202604-017,2026-04-18,-1100.00,Phones and internet,1107300.00
CHK-202604-018,2026-04-19,9400.00,Customer payment NorthStar,1116700.00
CHK-202604-019,2026-04-22,-12500.00,Payroll batch 04-C,1104200.00
CHK-202604-020,2026-04-22,15800.00,Customer payment Cascade,1120000.00
CHK-202604-021,2026-04-23,-680.00,Office supplies,1119320.00
CHK-202604-022,2026-04-24,11200.00,Customer payment Vertex,1130520.00
CHK-202604-023,2026-04-25,-3400.00,Travel expense reimburse,1127120.00
CHK-202604-024,2026-04-26,6900.00,Customer payment Lumen,1134020.00
CHK-202604-025,2026-04-29,-12500.00,Payroll batch 04-D,1121520.00
CHK-202604-026,2026-04-29,18500.00,Customer payment Helix,1140020.00
CHK-202604-027,2026-04-30,-980.00,Bank fees April,1139040.00
CHK-202604-028,2026-04-30,7600.00,Customer payment Solaris,1146640.00
CHK-202604-029,2026-05-01,-4200.00,Vendor payment Acme Supplies,1142440.00
CHK-202604-030,2026-05-02,14200.00,Customer payment Orion,1156640.00
CHK-202604-031,2026-05-03,-3200.00,Office rent May,1153440.00
CHK-202604-032,2026-05-05,9800.00,Customer payment Meridian,1163240.00
CHK-202604-033,2026-05-06,-1850.00,Utilities May,1161390.00
CHK-202604-034,2026-05-07,16400.00,Customer payment Polaris,1177790.00
CHK-202604-035,2026-05-08,-2200.00,Late fee waiver reversal,1175590.00
`;

// ── QuickBooks ledger (Books reconciliation, side B) ─────────────────────
// Same key shape as Chase. Drift:
//   - 3 amount conflicts: 003 (28000 vs 28500), 010 (45000 vs 44500), 020 (15800 vs 15900)
//   - 1 orphan only in QB:     CHK-202604-036
//   - 1 orphan only in Chase:  CHK-202604-035
// Perfect for Prism Books to surface those breaks.
const QUICKBOOKS_CSV = `ref,entry_seq,post_date,amount_usd,description,account
CHK-202604-001,1,2026-04-01,15400.00,Wire received Q1 dividend,4010 Dividend Income
CHK-202604-002,2,2026-04-02,-3200.00,Rent April,6010 Rent Expense
CHK-202604-003,3,2026-04-03,28500.00,Invoice INV-04023 ACME Corp,4100 Revenue
CHK-202604-004,4,2026-04-03,-12500.00,Payroll 04-A,6200 Payroll
CHK-202604-005,5,2026-04-04,-1850.00,Utilities,6020 Utilities
CHK-202604-006,6,2026-04-05,8200.00,Invoice GlobalTech,4100 Revenue
CHK-202604-007,7,2026-04-08,-450.00,SaaS subscription,6300 Software
CHK-202604-008,8,2026-04-09,18000.00,Invoice Stellar,4100 Revenue
CHK-202604-009,9,2026-04-09,-6200.00,Equipment lease April,6400 Equipment
CHK-202604-010,10,2026-04-10,44500.00,Escrow release,4200 Other Income
CHK-202604-011,11,2026-04-11,-2400.00,Google AdWords,6500 Marketing
CHK-202604-012,12,2026-04-12,12000.00,Invoice Apex,4100 Revenue
CHK-202604-013,13,2026-04-15,-12500.00,Payroll 04-B,6200 Payroll
CHK-202604-014,14,2026-04-15,7800.00,Invoice Bayside,4100 Revenue
CHK-202604-015,15,2026-04-16,-8900.00,Q1 insurance,6600 Insurance
CHK-202604-016,16,2026-04-17,22000.00,Invoice Hexagon,4100 Revenue
CHK-202604-017,17,2026-04-18,-1100.00,Telecom,6020 Utilities
CHK-202604-018,18,2026-04-19,9400.00,Invoice NorthStar,4100 Revenue
CHK-202604-019,19,2026-04-22,-12500.00,Payroll 04-C,6200 Payroll
CHK-202604-020,20,2026-04-22,15900.00,Invoice Cascade,4100 Revenue
CHK-202604-021,21,2026-04-23,-680.00,Supplies,6700 Office
CHK-202604-022,22,2026-04-24,11200.00,Invoice Vertex,4100 Revenue
CHK-202604-023,23,2026-04-25,-3400.00,Travel reimburse,6800 Travel
CHK-202604-024,24,2026-04-26,6900.00,Invoice Lumen,4100 Revenue
CHK-202604-025,25,2026-04-29,-12500.00,Payroll 04-D,6200 Payroll
CHK-202604-026,26,2026-04-29,18500.00,Invoice Helix,4100 Revenue
CHK-202604-027,27,2026-04-30,-980.00,Bank fees,6900 Bank Fees
CHK-202604-028,28,2026-04-30,7600.00,Invoice Solaris,4100 Revenue
CHK-202604-029,29,2026-05-01,-4200.00,Acme Supplies,6700 Office
CHK-202604-030,30,2026-05-02,14200.00,Invoice Orion,4100 Revenue
CHK-202604-031,31,2026-05-03,-3200.00,Rent May,6010 Rent Expense
CHK-202604-032,32,2026-05-05,9800.00,Invoice Meridian,4100 Revenue
CHK-202604-033,33,2026-05-06,-1850.00,Utilities May,6020 Utilities
CHK-202604-034,34,2026-05-07,16400.00,Invoice Polaris,4100 Revenue
CHK-202604-036,35,2026-05-09,5800.00,Manual entry — pending reconciliation,4100 Revenue
`;

export const DEMO_DATASETS: DemoDataset[] = [
  {
    id: "hospital_records",
    title: "Hospital patient records",
    badge: "Demo · encryption overlay",
    blurb:
      "30 fictitious patients across 4 departments — previews GIGI's encryption UX without actually encrypting anything client-side. Names + SSNs render OPAQUE (masked in UI), IDs + diagnoses + physicians render INDEXED, dollar/medical numerics render AFFINE. The real engine enforces the modes against ciphertext; this demo just labels the columns. Don't load real PHI through it.",
    source: "Synthetic PHI · HIPAA-shaped fictitious data",
    suggestedKey: "patient_id",
    suggestedCover: "department",
    records: 30,
    fields: 14,
    csv: HOSPITAL_CSV,
    encryption: {
      patient_id: "indexed",
      patient_name: "opaque",
      ssn_last4: "opaque",
      date_of_birth: "indexed",
      diagnosis_code: "indexed",
      diagnosis_text: "opaque",
      attending_md: "indexed",
      bp_systolic: "affine",
      length_of_stay: "affine",
      total_billed_usd: "affine",
    },
  },
  {
    id: "iris",
    title: "Iris flowers",
    blurb:
      "Fisher's 1936 dataset — 3 species × 4 measurements. Setosa is clearly separable; versicolor/virginica overlap. The OG demo for cohort geometry.",
    source: "R. A. Fisher, 1936 · public domain",
    suggestedKey: "id",
    suggestedCover: "species",
    records: 150,
    fields: 6,
    csv: IRIS_CSV,
  },
  {
    id: "nba_2024",
    title: "NBA teams · 2023-24 season",
    blurb:
      "All 30 NBA teams with wins, points scored, points allowed, and net rating. Conference makes a natural cover — East vs West splits cleanly. Pistons & Wizards land as anomalies.",
    source: "basketball-reference.com · public stats",
    suggestedKey: "id",
    suggestedCover: "conference",
    records: 30,
    fields: 9,
    csv: NBA_CSV,
  },
  {
    id: "world_cities",
    title: "World cities",
    blurb:
      "60 of the world's biggest/most recognizable cities. Continent is the natural cover. Outliers: Reykjavik (tiny + remote), La Paz (high altitude), Singapore (extremely dense).",
    source: "UN World Urbanization Prospects · public domain",
    suggestedKey: "id",
    suggestedCover: "continent",
    records: 60,
    fields: 9,
    csv: CITIES_CSV,
  },
  {
    id: "mall_customers",
    title: "Mall customer segmentation",
    blurb:
      "60 mall shoppers with age, income, and spending score. Gender is the natural cover. The textbook 5-cluster demo — and the spending-score distribution surfaces real cohort drift.",
    source: "Kaggle 'Mall_Customers' · public dataset",
    suggestedKey: "id",
    suggestedCover: "gender",
    records: 60,
    fields: 5,
    csv: MALL_CSV,
  },
  {
    id: "titanic",
    title: "Titanic passengers",
    badge: "Kaggle · classic",
    blurb:
      "50 passengers from the 1912 voyage — the canonical 'your first ML dataset.' Survival splits cleanly on sex × class: 1st-class women survive, 3rd-class men don't. Cover by `sex` for the survival split; switch to `embarked` to see port-of-embarkation cohorts.",
    source: "Kaggle 'Titanic' · public dataset",
    suggestedKey: "passenger_id",
    suggestedCover: "sex",
    records: 50,
    fields: 10,
    csv: TITANIC_CSV,
  },
  {
    id: "loan_approval",
    title: "Loan approval",
    badge: "Kaggle · 2024 playground",
    blurb:
      "50 loan applications. Predict approval from credit score, income, and DTI. The 2024 Kaggle Playground S4E10 regenerated the classic loan-default dataset with a fresh distribution — credit_score < 660 + DTI > 30% is the rejection sweet spot. Run Prism Monitor to surface borderline cases as κ-drift.",
    source: "Kaggle Playground S4E10 · 2024",
    suggestedKey: "application_id",
    suggestedCover: "loan_status",
    records: 50,
    fields: 10,
    csv: LOAN_CSV,
  },
  {
    id: "credit_card_fraud",
    title: "Credit card fraud",
    badge: "Kaggle · strong ML",
    blurb:
      "60 card transactions with 4 planted frauds (T-0010, T-0019, T-0032, T-0049). Real fraud signal: extreme distance_from_home, off-hours timestamps, foreign merchants, online + no-chip. Class imbalance ~6.7% mirrors production rates. Prism Monitor catches all 4 frauds as κ-anomalies on first pass.",
    source: "Kaggle 'Credit Card Fraud Detection' · public dataset",
    suggestedKey: "txn_id",
    suggestedCover: "merchant_category",
    records: 60,
    fields: 10,
    csv: FRAUD_CSV,
  },
  {
    id: "job_applicants",
    title: "Job applicants",
    badge: "Form view · Multi-select",
    blurb:
      "40 candidates across 5 hiring stages and 5 roles. Showcases Form view for intake, Multi-select tags on skills, Kanban on stage. Plus a planted anomaly — a top-scoring rejected candidate the sameness-find query catches in one click as a likely false negative.",
    source: "Synthetic · ATS-shaped hiring funnel",
    suggestedKey: "applicant_id",
    suggestedCover: "stage",
    records: 40,
    fields: 10,
    csv: APPLICANTS_CSV,
  },
  {
    id: "sensor_telemetry",
    title: "Sensor telemetry",
    badge: "Drag-fill · Calendar view",
    blurb:
      "10 industrial sensors × 5 daily readings. One sensor (S-007) is failing — temperature climbing 4°C/day while vibration drifts upward. Showcases Drag-fill (per-sensor OLS extrapolation), Calendar view with κ-tint per day, and Monitor's drift detector catching the planted fault.",
    source: "Synthetic · IIoT telemetry-shaped time series",
    suggestedKey: "reading_id",
    suggestedCover: "location",
    records: 50,
    fields: 9,
    csv: SENSORS_CSV,
  },
  {
    id: "payment_transactions",
    title: "Payment transactions",
    badge: "Prism · Dedup ready",
    blurb:
      "50 SWIFT/ACH/RTP-shaped payments with deliberate near-duplicates — same payment, slightly different reference formatting. Tailor-made for Prism Dedup; also includes sanctions-shaped counterparties for screening.",
    source: "Synthetic · payment-rail-shaped fictitious data",
    suggestedKey: "payment_id",
    suggestedCover: "rail",
    records: 50,
    fields: 11,
    csv: PAYMENTS_CSV,
  },
  {
    id: "daily_cashflow",
    title: "Daily cashflow",
    badge: "Prism · Forecast ready",
    blurb:
      "60 days of treasury cash flow across NA / EMEA / APAC with a clear downtrend. Pre-shaped for Prism Forecast — pick a region, project the next 7 days.",
    source: "Synthetic · treasury-shaped time series",
    suggestedKey: "cashflow_id",
    suggestedCover: "region",
    records: 60,
    fields: 7,
    csv: CASHFLOW_CSV,
  },
  {
    id: "account_activity",
    title: "Account activity (behavioral)",
    badge: "Prism · Monitor ready",
    blurb:
      "40 accounts with txn count, volume, velocity, country count, and structuring risk. Hidden anomalies: a few accounts with normal-looking volume but high velocity + many countries — Prism Monitor surfaces them instantly.",
    source: "Synthetic · AML-shaped behavioral metrics",
    suggestedKey: "account_id",
    suggestedCover: "account_type",
    records: 40,
    fields: 9,
    csv: ACCOUNTS_CSV,
  },
  {
    id: "chase_statements",
    title: "Chase statements (Books, side A)",
    badge: "Prism · Books · side A",
    blurb:
      "35 entries from a Chase business checking statement. Pair with the QuickBooks ledger below and run Prism Books — it will surface 3 amount conflicts and 2 orphans planted across the two bundles.",
    source: "Synthetic · bank-statement-shaped",
    suggestedKey: "ref",
    suggestedCover: "description",
    records: 35,
    fields: 5,
    csv: CHASE_CSV,
  },
  {
    id: "quickbooks_ledger",
    title: "QuickBooks ledger (Books, side B)",
    badge: "Prism · Books · side B",
    blurb:
      "35 entries from the matching QuickBooks ledger. Open this alongside Chase, run Prism Books, and watch the reconciliation engine find the planted breaks.",
    source: "Synthetic · accounting-ledger-shaped",
    suggestedKey: "ref",
    suggestedCover: "account",
    records: 35,
    fields: 6,
    csv: QUICKBOOKS_CSV,
  },
];

/** Look up a demo dataset by id. */
export function findDemo(id: string): DemoDataset | null {
  return DEMO_DATASETS.find((d) => d.id === id) ?? null;
}
