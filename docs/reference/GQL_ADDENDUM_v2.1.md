# GQL v2.1 Addendum — PostgreSQL Parity Features

**Date:** March 17, 2026
**Author:** Bee Rosa Davis · Davis Geometric
**Status:** Specification addendum to GQL v2.0

This document covers the features needed to bring GQL to full PostgreSQL parity.
The core geometric operations (50+) are in the v2.0 spec. This addendum covers
the "normal database plumbing" that PostgreSQL provides and GQL needs for
production readiness.

---

## 1. Access Control & Security

### WEAVE — Create roles (users/groups)

```sql
-- Create a role
WEAVE ROLE analyst;
WEAVE ROLE admin;
WEAVE ROLE readonly;

-- Role with password
WEAVE ROLE api_user PASSWORD 'secure_hash_here';

-- Role inheritance (admin inherits analyst permissions)
WEAVE ROLE admin INHERITS analyst;

-- Superuser equivalent
WEAVE ROLE superadmin SUPERWEAVE;

-- Drop a role
UNWEAVE ROLE analyst;

-- List roles
SHOW ROLES;
```

### GRANT / REVOKE — Permissions

```sql
-- Grant specific operations
GRANT SECTION ON sensors TO admin;           -- insert
GRANT COVER ON sensors TO analyst;           -- read (range)
GRANT REDEFINE ON sensors TO admin;          -- update
GRANT RETRACT ON sensors TO admin;           -- delete
GRANT CURVATURE ON sensors TO analyst;       -- geometric analysis
GRANT SPECTRAL ON sensors TO analyst;
GRANT HEALTH ON sensors TO analyst;

-- Grant all
GRANT ALL ON sensors TO admin;

-- Grant on all bundles
GRANT COVER ON ALL BUNDLES TO readonly;

-- Revoke
REVOKE RETRACT ON sensors FROM analyst;
REVOKE ALL ON sensors FROM readonly;

-- Grant schema operations
GRANT BUNDLE TO admin;                       -- can create bundles
GRANT GAUGE ON sensors TO admin;             -- can modify schema
GRANT COLLAPSE ON sensors TO admin;          -- can drop bundles
```

### POLICY — Row-level security via geometric restriction

```sql
-- Analyst can only see EU data
POLICY analyst_eu ON sensors
  FOR COVER
  RESTRICT TO (COVER sensors ON region = 'EU')
  TO analyst;

-- Multiple policies (OR logic — if any policy passes, access granted)
POLICY analyst_na ON sensors
  FOR COVER
  RESTRICT TO (COVER sensors ON region = 'NA')
  TO analyst;

-- Write restriction
POLICY admin_eu ON sensors
  FOR SECTION, REDEFINE, RETRACT
  RESTRICT TO (COVER sensors ON region = 'EU')
  TO admin;

-- Curvature-based access (GQL-only — no Postgres equivalent)
POLICY high_confidence ON sensors
  FOR COVER
  RESTRICT TO (COVER sensors CONFIDENCE >= 0.95)
  TO analyst;
-- Analyst can only see records from low-curvature regions.
-- The geometry determines access. Wild.

-- Drop policy
DROP POLICY analyst_eu ON sensors;

-- Show policies
SHOW POLICIES ON sensors;
```

### AUDIT — Audit trail

```sql
-- Enable auditing on a bundle
AUDIT sensors ON;

-- Audit with specific tracking
AUDIT sensors ON SECTION, REDEFINE, RETRACT, GAUGE;

-- View audit log
AUDIT SHOW sensors;
AUDIT SHOW sensors SINCE '2024-01-01';
AUDIT SHOW sensors ROLE admin;

-- Disable
AUDIT sensors OFF;
```

---

## 2. Constraints

### Fiber Value Constraints

```sql
BUNDLE orders
  BASE (id NUMERIC AUTO)
  FIBER (
    -- Simple constraints inline
    customer_id NUMERIC REQUIRED,
    total NUMERIC REQUIRED CHECK (total > 0),
    qty NUMERIC CHECK (qty >= 1 AND qty <= 10000),
    status CATEGORICAL IN ('pending', 'shipped', 'delivered', 'cancelled'),
    email TEXT CHECK (email MATCHES '/^[^@]+@[^@]+\.[^@]+$/'),
    discount NUMERIC CHECK (discount >= 0 AND discount <= 1),
    
    -- Foreign key (bundle morphism)
    customer_id NUMERIC REFERENCES customers(id),
    region_id NUMERIC REFERENCES regions(id)
  );
```

### CONSTRAINT block — Cross-field and composite constraints

```sql
BUNDLE orders
  BASE (id NUMERIC AUTO)
  FIBER (...)
  CONSTRAINT (
    -- Composite uniqueness
    UNIQUE (customer_id, date),
    
    -- Cross-field constraint
    CHECK (ship_date >= order_date),
    CHECK (discount_total <= total),
    CHECK (qty * unit_price = line_total),
    
    -- Conditional constraint
    CHECK (status = 'shipped' IMPLIES ship_date DEFINED),
    CHECK (status = 'cancelled' IMPLIES cancel_reason DEFINED),
    
    -- Bundle morphism (explicit foreign key with geometric semantics)
    MORPHISM customer_id -> customers(id),
    MORPHISM region_id -> regions(id),
    
    -- Exclusion constraint (no overlapping date ranges per customer)
    EXCLUDE (customer_id WITH =, daterange WITH OVERLAPS)
  );
```

### Adding/Dropping constraints after creation

```sql
-- Add constraint
GAUGE orders CONSTRAIN (
  ADD CHECK (total > 0) AS positive_total,
  ADD UNIQUE (customer_id, date) AS unique_order,
  ADD MORPHISM region_id -> regions(id)
);

-- Drop constraint by name
GAUGE orders UNCONSTRAIN positive_total;

-- Show constraints
SHOW CONSTRAINTS ON orders;
```

### Geometric constraint semantics

Constraints in GQL are sections of a constraint bundle. A constraint violation
means the proposed section does not exist in the constraint fiber. This is
checkable at insert time in O(1) for value constraints and O(1) for
MORPHISM constraints (pullback lookup).

```
Constraint violation flow:
  SECTION orders (total: -5, ...)
  → Check fiber constraint: total > 0
  → -5 is not in the constraint fiber {x : x > 0}
  → REJECT: "Constraint 'positive_total' violated: total = -5, expected > 0"
  → Section NOT stored. Curvature NOT updated.
```

---

## 3. Built-in Functions

### Math Functions (fiber arithmetic)

```sql
COVER sensors PROJECT (
  city, temp,
  ABS(temp) AS abs_temp,                    -- absolute value
  ROUND(temp, 1) AS rounded,                -- round to n decimals
  CEIL(wind) AS wind_ceil,                   -- ceiling
  FLOOR(humidity) AS humid_floor,            -- floor
  TRUNC(temp, 0) AS truncated,              -- truncate decimals
  POWER(temp, 2) AS temp_squared,            -- exponentiation
  SQRT(ABS(temp)) AS temp_root,              -- square root
  CBRT(ABS(temp)) AS cube_root,              -- cube root
  LOG(pressure) AS log_p,                    -- natural log
  LOG10(pressure) AS log10_p,                -- base-10 log
  LOG2(pressure) AS log2_p,                  -- base-2 log
  EXP(temp / 100) AS exp_scaled,             -- e^x
  MOD(id, 3) AS bucket,                      -- modulo
  SIGN(temp) AS direction,                   -- -1, 0, or 1
  GREATEST(temp, 0) AS temp_floor_zero,      -- max of values
  LEAST(temp, 40) AS temp_cap,               -- min of values
  PI() AS pi_val,                            -- π
  RANDOM() AS rand                           -- random [0,1)
);
```

### String Functions (fiber text operations)

```sql
COVER sensors PROJECT (
  UPPER(city) AS city_upper,                 -- uppercase
  LOWER(city) AS city_lower,                 -- lowercase
  INITCAP(city) AS city_title,               -- title case
  LENGTH(city) AS name_len,                  -- character count
  SUBSTR(city, 1, 3) AS prefix,              -- substring
  LEFT(city, 3) AS left3,                    -- leftmost n chars
  RIGHT(city, 3) AS right3,                  -- rightmost n chars
  CONCAT(city, '-', region) AS label,        -- concatenation
  CONCAT_WS(', ', city, region) AS full,     -- concat with separator
  REPLACE(status, 'normal', 'ok') AS stat,   -- replace
  TRIM(city) AS trimmed,                     -- trim whitespace
  LTRIM(city) AS left_trimmed,
  RTRIM(city) AS right_trimmed,
  LPAD(id, 5, '0') AS padded_id,            -- left pad: "00042"
  RPAD(city, 15, '.') AS dotted,             -- right pad
  REVERSE(city) AS reversed,                 -- reverse string
  REPEAT('*', 3) AS stars,                   -- repeat string
  POSITION('ow' IN city) AS pos,             -- find position (0 if not found)
  SPLIT(city, '_') AS parts,                 -- split to array
  MD5(city) AS hash                          -- MD5 hash (for checksums, not security)
);
```

### Date/Time Functions (base space arithmetic)

```sql
COVER events PROJECT (
  timestamp,
  NOW() AS current_time,                     -- current timestamp
  TODAY() AS current_date,                   -- current date
  EPOCH(timestamp) AS unix_ts,               -- to Unix epoch seconds
  FROM_EPOCH(1710000000) AS ts,              -- from Unix epoch
  DATEPART(timestamp, 'year') AS yr,         -- extract component
  DATEPART(timestamp, 'month') AS mo,
  DATEPART(timestamp, 'day') AS dy,
  DATEPART(timestamp, 'hour') AS hr,
  DATEPART(timestamp, 'dow') AS day_of_week, -- 0=Sun, 6=Sat
  DATEPART(timestamp, 'doy') AS day_of_year,
  DATETRUNC(timestamp, 'month') AS month_start,  -- truncate to unit
  DATETRUNC(timestamp, 'day') AS day_start,
  DATEADD(timestamp, 7, 'day') AS next_week,     -- add interval
  DATEADD(timestamp, -1, 'month') AS last_month,
  DATEDIFF(end_ts, start_ts, 'hours') AS duration_hrs,  -- difference
  DATEDIFF(end_ts, start_ts, 'days') AS duration_days,
  FORMAT_DATE(timestamp, 'YYYY-MM-DD') AS formatted,     -- format
  PARSE_DATE('2024-01-04', 'YYYY-MM-DD') AS parsed       -- parse
);
```

### Type Casting (fiber morphism)

```sql
COVER sensors PROJECT (
  CAST(temp AS TEXT) AS temp_str,
  CAST('42' AS NUMERIC) AS num,
  CAST(temp AS INTEGER) AS temp_int,
  CAST(1 AS BOOLEAN) AS flag,
  CAST(date AS TIMESTAMP) AS ts
);

-- Shorthand
COVER sensors PROJECT (
  temp::TEXT AS temp_str,
  '42'::NUMERIC AS num
);
```

### Conditional Expressions

```sql
COVER sensors PROJECT (
  city, temp,
  
  -- CLASSIFY (extended CASE/WHEN — already in v2.0)
  CLASSIFY temp
    WHEN temp < -20 THEN 'extreme_cold'
    WHEN temp < 0 THEN 'freezing'
    WHEN temp < 15 THEN 'cold'
    WHEN temp < 25 THEN 'mild'
    WHEN temp < 35 THEN 'warm'
    ELSE 'extreme_heat'
  AS category,
  
  -- IF (simple two-branch)
  IF(temp < 0, 'below_zero', 'above_zero') AS freezing,
  
  -- RESOLVE (COALESCE equivalent — already in v2.0)
  RESOLVE(pressure, 101.3) AS pressure_safe,
  
  -- NULLIF equivalent: return VOID if value matches
  VOIDIF(wind, 0) AS wind_nonzero,
  
  -- GREATEST / LEAST
  GREATEST(temp, temp_min, 0) AS warmest,
  LEAST(temp, temp_max, 40) AS coolest
);
```

### Array/Collection Functions

```sql
-- Array construction
COVER sensors PROJECT (
  city,
  ARRAY(temp, humidity, wind) AS measurements,
  ARRAY_LENGTH(ARRAY(temp, humidity, wind)) AS n_measures
);

-- Array aggregation
INTEGRATE sensors OVER city MEASURE
  ARRAY_AGG(temp) AS all_temps,              -- collect values into array
  ARRAY_AGG(temp RANK BY date ASC) AS ordered_temps;  -- ordered array

-- JSON construction
COVER sensors PROJECT (
  JSON_BUILD(
    'city', city,
    'temp', temp,
    'confidence', CONFIDENCE()
  ) AS record_json
);
```

---

## 4. Conditional Aggregation — FILTER Clause

```sql
INTEGRATE sensors OVER city MEASURE
  count(*) AS total,
  count(*) FILTER (WHERE temp < 0) AS freezing_days,
  count(*) FILTER (WHERE temp > 30) AS hot_days,
  avg(temp) FILTER (WHERE status = 'alert') AS alert_avg_temp,
  max(wind) FILTER (WHERE status = 'alert') AS alert_max_wind,
  stddev(temp) FILTER (WHERE DATEPART(date, 'month') IN (12, 1, 2)) AS winter_stddev;

-- FILTER with curvature (GQL-only)
INTEGRATE sensors OVER city MEASURE
  avg(temp) FILTER (WHERE CURVATURE(temp) > 0.01) AS volatile_avg,
  count(*) FILTER (WHERE CONFIDENCE() < 0.95) AS low_confidence_count;
```

---

## 5. Recursive Queries — ITERATE

### WITH RECURSIVE equivalent

```sql
-- SQL: WITH RECURSIVE org AS (
--        SELECT * FROM employees WHERE id = 1
--        UNION ALL
--        SELECT e.* FROM employees e JOIN org ON e.manager_id = org.id
--      )

-- GQL:
WITH RECURSIVE chain AS (
  SECTION employees AT id=1
  UNION
  PULLBACK chain ALONG manager_id ONTO employees
)
COVER chain;
```

### ITERATE — Clean syntax for recursive transport

```sql
-- Walk a hierarchy
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID;
-- Returns: all reachable sections via iterated pullback along manager_id
-- Terminates when pullback returns no match (VOID)

-- Limit depth
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID
  MAX DEPTH 10;

-- With accumulation (compute at each level)
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID
  ACCUMULATE count(*) AS team_size, sum(salary) AS total_salary;

-- Graph traversal (follow any indexed field)
ITERATE friends
  START AT user_id=42
  STEP ALONG friend_id
  UNTIL DEPTH 3;
-- 3-hop friend-of-friend network
-- Returns: all sections reachable in ≤ 3 pullback steps
```

---

## 6. RETURNING — Get back what you wrote

```sql
-- Insert with return
SECTION sensors (id: 99, city: 'Test', region: 'XX', date: 20260317,
  temp: 22.0, humidity: 50.0, wind: 3.0)
RETURNING *;
-- Returns: the stored section + confidence + curvature + anomaly flag

-- Return specific fields
SECTION sensors (...) RETURNING id, confidence, curvature;

-- Update with return (old and new)
REDEFINE sensors AT id=42 SET (temp: -28.5)
RETURNING temp AS new_temp, OLD.temp AS old_temp, curvature;

-- Delete with return (what was deleted)
RETRACT sensors AT id=42 RETURNING *;
-- Returns the retracted section so you know what was removed

-- Batch with return
SECTIONS sensors (..., ..., ...)
RETURNING id, anomaly;
-- Returns per-record anomaly flags for the batch
```

---

## 7. Generate Series — GENERATE BASE

```sql
-- Generate base points for a time series (fill the base space)
GENERATE BASE sensors
  FROM date=20240101 TO date=20241231 STEP 1;
-- Creates section stubs at every base point in the sequence
-- Sections with no data have VOID fibers

-- With a specific city
GENERATE BASE sensors
  FROM date=20240101 TO date=20241231 STEP 1
  WITH (city: 'Moscow', region: 'EU');
-- Pre-populates city and region, leaves other fibers VOID

-- Detect gaps (dates with no data)
COVER sensors ON city = 'Moscow' WHERE temp VOID;
-- Returns all Moscow dates where temperature was not recorded

-- Fill gaps via parallel transport
FILL sensors ON date
  USING TRANSPORT;
-- For each VOID section, parallel transports the nearest non-VOID value
-- Curvature-aware: adjusts for local variability

-- Fill with interpolation
FILL sensors ON date
  USING INTERPOLATE LINEAR;
-- Linear interpolation between surrounding non-VOID values

-- Fill with constant
FILL sensors ON date
  USING CONSTANT (temp: 0.0, humidity: 50.0);
-- Fill all VOID fibers with specified values

-- Generate integer sequence (for testing / synthetic data)
GENERATE BASE test_bundle
  FROM id=1 TO id=1000000 STEP 1;
```

---

## 8. Bulk Import/Export — INGEST and EMIT TO

### INGEST — Bulk import from files

```sql
-- CSV import
INGEST sensors FROM 'data.csv' FORMAT CSV;
INGEST sensors FROM 'data.csv' FORMAT CSV
  HEADER TRUE
  DELIMITER ','
  NULL_VALUE 'NA'
  SKIP 1;

-- JSON import
INGEST sensors FROM 'data.json' FORMAT JSON;
INGEST sensors FROM 'data.jsonl' FORMAT JSONL;  -- line-delimited JSON

-- DHOOM import (native)
INGEST sensors FROM 'data.dhoom' FORMAT DHOOM;

-- SQL dump import
INGEST sensors FROM 'dump.sql' FORMAT SQL;

-- From stdin (piping)
INGEST sensors FROM STDIN FORMAT CSV;
INGEST sensors FROM STDIN FORMAT JSONL;

-- From URL
INGEST sensors FROM 'https://api.nasa.gov/power/...' FORMAT JSON
  PATH 'properties.parameter';

-- With transformation on ingest
INGEST sensors FROM 'raw.csv' FORMAT CSV
  MAP (
    temperature -> temp,
    relative_humidity -> humidity,
    wind_speed -> wind
  )
  FILTER (WHERE temp DEFINED AND humidity DEFINED);

-- Import report
-- Response:
-- INGEST COMPLETE: 7320 sections stored
-- rejected: 12 (constraint violations)
-- anomalies_detected: 47
-- curvature_after: 0.0346
-- elapsed: 28ms (256K sections/sec)
```

### EMIT TO — Bulk export to files

```sql
-- Export to file
COVER sensors ALL EMIT CSV TO 'export.csv';
COVER sensors ALL EMIT JSON TO 'export.json';
COVER sensors ALL EMIT DHOOM TO 'export.dhoom';

-- Export filtered data
COVER sensors ON city = 'Moscow' WHERE temp < -25
  EMIT CSV TO 'moscow_cold.csv';

-- Export with geometric metadata
COVER sensors ALL
  EMIT CSV TO 'export_with_meta.csv'
  WITH CURVATURE, CONFIDENCE;
-- Adds K and confidence columns to CSV output

-- Export to stdout (for piping)
COVER sensors ON region = 'EU' EMIT CSV TO STDOUT;
COVER sensors ON region = 'EU' EMIT JSONL TO STDOUT;

-- Export aggregation results
INTEGRATE sensors OVER city MEASURE avg(temp), count(*)
  EMIT CSV TO 'city_stats.csv';

-- DHOOM export preserves full bundle structure
COVER sensors ALL EMIT DHOOM TO 'full_backup.dhoom';
-- This is a complete geometric backup: schema, defaults, sections, metadata
```

---

## 9. Prepared Statements — PREPARE / EXECUTE

```sql
-- Prepare a parameterized point query
PREPARE point_lookup AS
  SECTION sensors AT id = $1;

EXECUTE point_lookup (42);
EXECUTE point_lookup (43);
EXECUTE point_lookup (100);

-- Prepare a parameterized cover
PREPARE city_query AS
  COVER sensors ON city = $1 WHERE temp < $2;

EXECUTE city_query ('Moscow', -25);
EXECUTE city_query ('Toronto', -5);

-- Prepare aggregation
PREPARE region_stats AS
  INTEGRATE sensors OVER city
    MEASURE avg(temp), count(*)
    RESTRICT TO (COVER sensors ON region = $1);

EXECUTE region_stats ('EU');
EXECUTE region_stats ('AS');

-- Prepare with multiple parameters
PREPARE range_query AS
  COVER sensors ON region = $1 WHERE temp > $2 AND wind < $3
    RANK BY temp DESC FIRST $4;

EXECUTE range_query ('EU', 20, 5, 10);

-- Deallocate
DEALLOCATE point_lookup;
DEALLOCATE ALL;

-- Show prepared statements
SHOW PREPARED;
```

---

## 10. Triggers & Event Hooks — ON ... EXECUTE

```sql
-- Trigger on insert
ON SECTION sensors
  EXECUTE NOTIFY 'new_reading';

-- Trigger on insert with condition
ON SECTION sensors
  WHERE temp < -30
  EXECUTE ALERT 'extreme_cold';

-- Trigger on insert with condition and action
ON SECTION sensors
  WHERE status = 'alert'
  EXECUTE (
    SECTION alerts (
      source: 'sensors',
      record_id: NEW.id,
      message: CONCAT('Alert from ', NEW.city, ': temp=', NEW.temp),
      timestamp: NOW()
    )
  );

-- Trigger on curvature drift
ON CURVATURE sensors
  WHERE DRIFT(temp) > 0.01
  EXECUTE NOTIFY 'volatility_shift';

-- Trigger on consistency violation
ON CONSISTENCY sensors
  WHERE h1 > 0
  EXECUTE REPAIR;
-- Auto-repair when Čech H¹ goes nonzero

-- Cascade delete (trigger on retract)
ON RETRACT customers
  CASCADE RETRACT orders WHERE customer_id = OLD.id;

-- Cascade via morphism (automatic — if MORPHISM declared, cascade follows)
ON RETRACT customers
  CASCADE MORPHISM;
-- Retracts all sections in orders where customer_id morphism points to deleted customer

-- Before-trigger (validation)
BEFORE SECTION orders
  CHECK (
    EXISTS SECTION customers AT id = NEW.customer_id
  )
  ON FAIL REJECT 'Customer does not exist';

-- After-trigger (logging)
AFTER REDEFINE sensors
  EXECUTE (
    SECTION audit_log (
      bundle: 'sensors',
      action: 'REDEFINE',
      record_id: OLD.id,
      changed_fields: DIFF(OLD, NEW),
      timestamp: NOW(),
      role: CURRENT_ROLE()
    )
  );

-- Drop trigger
DROP TRIGGER extreme_cold ON sensors;

-- Show triggers
SHOW TRIGGERS ON sensors;
```

---

## 11. Backup & Restore

```sql
-- Full bundle backup (schema + data + geometric state)
BACKUP sensors TO 'sensors_2024.gigi';

-- All bundles
BACKUP ALL TO 'full_backup_2024.gigi';

-- Backup with compression
BACKUP sensors TO 'sensors.gigi.zst' COMPRESS;

-- Incremental backup (only changes since last backup)
BACKUP sensors TO 'sensors_incr.gigi' INCREMENTAL SINCE '2024-06-01';

-- Restore
RESTORE sensors FROM 'sensors_2024.gigi';

-- Restore to a specific snapshot
RESTORE sensors FROM 'sensors_2024.gigi' AT SNAPSHOT 'pre_migration';

-- Restore with rename (don't overwrite existing)
RESTORE sensors FROM 'sensors_2024.gigi' AS sensors_restored;

-- Verify backup integrity
VERIFY BACKUP 'sensors_2024.gigi';
-- Returns: sections, schema hash, curvature at backup time, CRC check

-- List backups
SHOW BACKUPS;
```

---

## 12. Maintenance Operations

```sql
-- Force WAL compaction
COMPACT sensors;

-- Compact with statistics rebuild
COMPACT sensors ANALYZE;

-- Refresh geometric statistics (curvature, spectral, etc.)
ANALYZE sensors;
ANALYZE sensors ON temp;              -- specific field only
ANALYZE sensors FULL;                 -- all fields + cross-field metrics

-- Storage report
STORAGE sensors;
-- Returns:
-- base_geometry: SEQUENTIAL (array mode)
-- sections: 7320
-- memory_usage: 4.2 MB
-- wal_size: 1.8 MB
-- field_index_size: 0.3 MB
-- curvature_cache: 0.1 MB
-- total: 6.4 MB
-- compression_ratio: 3.2x vs raw JSON

-- Rebuild field index (if corrupted)
REBUILD INDEX sensors;
REBUILD INDEX sensors ON city;

-- Vacuum (reclaim space from retracted sections)
VACUUM sensors;
VACUUM sensors FULL;                  -- also defragments storage

-- Check integrity
CHECK sensors;
-- Verifies:
-- - WAL CRC integrity
-- - Field index consistency with sections
-- - GIGI hash uniqueness
-- - Constraint satisfaction across all sections
-- - Curvature cache accuracy (recompute and compare)
-- Returns: status, issues found, repair suggestions

-- Repair
REPAIR sensors;
-- Fixes: WAL corruption, field index desync, curvature cache staleness
```

---

## 13. Session & Connection Management

```sql
-- Set session-level configuration
SET TOLERANCE 0.01;                   -- default τ for this session
SET EMIT DHOOM;                       -- default output format
SET ANOMALY_THRESHOLD 2.5;            -- default σ threshold
SET MAX_SECTIONS 1000000;             -- query result limit

-- Show current settings
SHOW SETTINGS;

-- Session info
SHOW SESSION;
-- Returns: role, connected_at, bundles_accessed, queries_executed

-- Current role
SHOW CURRENT ROLE;

-- Reset to defaults
RESET TOLERANCE;
RESET ALL;
```

---

## 14. Information Schema — Bundle Metadata

```sql
-- All bundles with details
SHOW BUNDLES;
SHOW BUNDLES VERBOSE;

-- Schema for a specific bundle
DESCRIBE sensors;
DESCRIBE sensors VERBOSE;

-- Field-level metadata
SHOW FIELDS ON sensors;
-- Returns: name, type, modifiers, K, distinct_count, void_rate, index_size

-- Index information
SHOW INDEXES ON sensors;
-- Returns: field, distinct_values, bucket_sizes, bitmap_memory

-- Constraint information
SHOW CONSTRAINTS ON sensors;

-- Morphism (foreign key) information
SHOW MORPHISMS ON sensors;
-- Returns: field, target_bundle, target_field, referential_integrity_status

-- Trigger information
SHOW TRIGGERS ON sensors;

-- Policy information
SHOW POLICIES ON sensors;

-- Statistics
SHOW STATISTICS ON sensors;
-- Returns: per-field mean, stddev, min, max, K, entropy, distinct, void_rate

-- Geometric summary
SHOW GEOMETRY ON sensors;
-- Returns: scalar_K, spectral_gap, betti_numbers, euler_characteristic,
--          storage_mode, base_geometry, double_cover_status
```

---

## 15. Comments & Documentation

```sql
-- Add comment to a bundle
COMMENT ON BUNDLE sensors IS 'NASA POWER atmospheric data, 20 cities, 2024';

-- Add comment to a field
COMMENT ON FIELD sensors.temp IS 'Temperature at 2 meters (°C), RANGE 80 for K normalization';

-- Add comment to a constraint
COMMENT ON CONSTRAINT positive_total ON orders IS 'Prevents negative order totals';

-- Show comments
SHOW COMMENTS ON sensors;
```

---

## 16. System Functions

```sql
-- Current state
NOW()                                 -- current timestamp
TODAY()                               -- current date
CURRENT_ROLE()                        -- current role name
CURRENT_BUNDLE()                      -- last accessed bundle
VERSION()                             -- GIGI version string

-- Geometric state functions (usable in PROJECT or WHERE)
CONFIDENCE()                          -- confidence at current base point
CURVATURE()                           -- K at current base point
ANOMALY()                             -- boolean anomaly flag
Z_SCORE()                             -- z-score at current base point
CAPACITY()                            -- C = τ/K at current base point

-- Example: use geometric functions in queries
COVER sensors PROJECT (
  city, temp,
  CONFIDENCE() AS conf,
  CURVATURE() AS K,
  ANOMALY() AS is_anomaly,
  Z_SCORE() AS z
);

-- Filter by geometric function
COVER sensors WHERE ANOMALY() = TRUE;
COVER sensors WHERE Z_SCORE() > 3.0;
COVER sensors WHERE CONFIDENCE() < 0.90;
```

---

## 17. Error Handling

```sql
-- Try/catch in transactions
ATLAS BEGIN;
  SECTION orders (id: 1, customer_id: 999, total: 50.0);
  ON ERROR (
    CASE
      WHEN CONSTRAINT_VIOLATION THEN
        NOTIFY 'constraint_error'
      WHEN MORPHISM_VIOLATION THEN
        NOTIFY 'missing_customer'
      ELSE
        ATLAS ROLLBACK
    END
  );
ATLAS COMMIT;

-- Error info
SHOW LAST ERROR;
-- Returns: error_type, message, constraint_name, field, value, suggestion
```

---

## 18. Copy Between Bundles — TRANSPLANT

```sql
-- Copy sections from one bundle to another
TRANSPLANT sensors INTO sensors_archive;

-- Copy with filter
TRANSPLANT sensors INTO sensors_archive
  WHERE date < 20240601;

-- Copy with transformation
TRANSPLANT sensors INTO sensors_v2
  MAP (
    temp -> temperature,
    humidity -> relative_humidity
  );

-- Move (copy + retract from source)
TRANSPLANT sensors INTO sensors_archive
  WHERE date < 20240101
  RETRACT SOURCE;
```

---

## Complete PostgreSQL Parity Checklist

| Category | PostgreSQL | GQL v2.1 | Notes |
|---|---|---|---|
| CRUD | ✓ | ✓ | SECTION, REDEFINE, RETRACT, UPSERT |
| Range queries | ✓ | ✓ | COVER with ON/WHERE, DISTINCT, RANK, FIRST, SKIP |
| Aggregation | ✓ | ✓ | INTEGRATE with FILTER clause |
| Window functions | ✓ | ✓ | FIBER operations + TRANSPORT |
| Joins | ✓ | ✓ | PULLBACK + PRODUCT + set ops |
| Subqueries / CTEs | ✓ | ✓ | Nested covers + WITH + WITH RECURSIVE |
| Recursive queries | ✓ | ✓ | ITERATE (clean syntax for recursive pullback) |
| Transactions | ✓ | ✓ | ATLAS with FLAT/CURVED isolation |
| Constraints | ✓ | ✓ | CHECK, UNIQUE, REFERENCES, MORPHISM, IN, EXCLUDE |
| Access control | ✓ | ✓ | WEAVE, GRANT, REVOKE, POLICY |
| Row-level security | ✓ | ✓ | POLICY with geometric RESTRICT |
| Audit trail | ✓ | ✓ | AUDIT ON/OFF/SHOW |
| Math functions | ✓ | ✓ | ABS, ROUND, SQRT, LOG, POWER, MOD, etc. |
| String functions | ✓ | ✓ | UPPER, LOWER, SUBSTR, CONCAT, REPLACE, etc. |
| Date functions | ✓ | ✓ | EPOCH, DATEPART, DATETRUNC, DATEADD, DATEDIFF |
| Type casting | ✓ | ✓ | CAST, :: shorthand |
| Conditional expressions | ✓ | ✓ | CLASSIFY, IF, RESOLVE, VOIDIF, GREATEST, LEAST |
| RETURNING | ✓ | ✓ | On SECTION, REDEFINE, RETRACT |
| Conditional aggregation | ✓ | ✓ | FILTER (WHERE ...) on any aggregate |
| Generate series | ✓ | ✓ | GENERATE BASE + FILL |
| Bulk import | ✓ | ✓ | INGEST from CSV/JSON/JSONL/DHOOM/SQL/URL |
| Bulk export | ✓ | ✓ | EMIT TO file (CSV/JSON/DHOOM) |
| Prepared statements | ✓ | ✓ | PREPARE / EXECUTE / DEALLOCATE |
| Triggers | ✓ | ✓ | ON SECTION/REDEFINE/RETRACT + BEFORE/AFTER |
| Cascade operations | ✓ | ✓ | CASCADE RETRACT, CASCADE MORPHISM |
| Backup/restore | ✓ | ✓ | BACKUP/RESTORE with INCREMENTAL and COMPRESS |
| Maintenance | ✓ | ✓ | COMPACT, ANALYZE, VACUUM, REBUILD, CHECK, REPAIR |
| Session management | ✓ | ✓ | SET, SHOW, RESET |
| Information schema | ✓ | ✓ | SHOW BUNDLES/FIELDS/INDEXES/CONSTRAINTS/etc. |
| Comments | ✓ | ✓ | COMMENT ON BUNDLE/FIELD/CONSTRAINT |
| Error handling | ✓ | ✓ | ON ERROR in ATLAS blocks |
| Copy between tables | ✓ | ✓ | TRANSPLANT |
| Full-text search | ✓ | ✓ | MATCHES with wildcard and regex |
| Arrays | ✓ | ✓ | ARRAY(), ARRAY_AGG, ARRAY_LENGTH |
| JSON support | ✓ | ✓ | JSON_BUILD, EMIT JSON |
| System functions | ✓ | ✓ | NOW, VERSION, CURRENT_ROLE, CONFIDENCE, CURVATURE |
| Views | ✓ | ✓ | LENS + MATERIALIZE |
| **Geometric ops (50+)** | **✗** | **✓** | Curvature, spectral, topology, stat mech, gauge, transport |
| **Anomaly detection** | **✗** | **✓** | Built-in via curvature |
| **Confidence scoring** | **✗** | **✓** | 1/(1+K) on every query |
| **Consistency proof** | **✗** | **✓** | Čech H¹, holonomy, Wilson loops |
| **Wire compression** | **✗** | **✓** | DHOOM (50-84% smaller) |

**Final score: 37 PostgreSQL categories matched. 50+ GQL-only operations added. 0 gaps remaining.**

---

**GQL v2.1** · Geometric Query Language · Davis Geometric · 2026
