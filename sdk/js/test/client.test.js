/**
 * Test suite for @gigi-db/client SDK
 * Run: node --test test/client.test.js
 * Requires: GIGI Stream running on port 3142 (or set GIGI_URL)
 */

import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";

// Since we're testing the source directly (not built), import the TS source
// In production, users import from '@gigi-db/client'
const GIGI_URL = process.env.GIGI_URL || "http://localhost:3142";

// We'll use fetch directly for the test since we're testing the API contract
async function api(method, path, body) {
  const opts = {
    method,
    headers: { "Content-Type": "application/json" },
  };
  if (body) opts.body = JSON.stringify(body);
  const res = await fetch(`${GIGI_URL}${path}`, opts);
  if (!res.ok) throw new Error(`${res.status}: ${await res.text()}`);
  return res.json();
}

describe("GIGI Client SDK Contract", () => {
  before(async () => {
    // Verify server is running
    const health = await api("GET", "/v1/health");
    assert.equal(health.status, "ok");
    console.log(`  Connected to ${health.engine} v${health.version}`);
  });

  it("should create a bundle", async () => {
    const result = await api("POST", "/v1/bundles", {
      name: "sdk_test",
      schema: {
        fields: {
          id: "categorical",
          name: "categorical",
          score: "numeric",
          active: "categorical",
        },
        keys: ["id"],
        defaults: { active: "true" },
        indexed: ["active", "score"],
      },
    });
    assert.equal(result.status, "created");
    assert.equal(result.bundle, "sdk_test");
  });

  it("should insert records with curvature", async () => {
    const result = await api("POST", "/v1/bundles/sdk_test/insert", {
      records: [
        { id: "u-1", name: "Alice", score: 95, active: "true" },
        { id: "u-2", name: "Bob", score: 87, active: "true" },
        { id: "u-3", name: "Carol", score: 92, active: "false" },
        { id: "u-4", name: "Dave", score: 78, active: "true" },
        { id: "u-5", name: "Eve", score: 99, active: "true" },
      ],
    });
    assert.equal(result.status, "inserted");
    assert.equal(result.count, 5);
    assert.equal(result.total, 5);
    assert.ok(result.curvature >= 0, "curvature should be non-negative");
    assert.ok(
      result.confidence > 0 && result.confidence <= 1,
      "confidence should be in (0, 1]"
    );
    console.log(
      `  K=${result.curvature}, confidence=${result.confidence}`
    );
  });

  it("should point query O(1) with metadata", async () => {
    const result = await api("GET", "/v1/bundles/sdk_test/get?id=u-1");
    assert.equal(result.data.name, "Alice");
    assert.equal(result.data.score, 95);
    assert.ok(result.meta.confidence > 0);
    assert.ok(result.meta.curvature >= 0);
    assert.ok(result.meta.capacity > 0);
    console.log(
      `  Alice: score=${result.data.score}, confidence=${result.meta.confidence}`
    );
  });

  it("should range query", async () => {
    const result = await api(
      "GET",
      "/v1/bundles/sdk_test/range?active=false"
    );
    assert.equal(result.data.length, 1);
    assert.equal(result.data[0].name, "Carol");
    console.log(`  Found ${result.data.length} inactive user(s)`);
  });

  it("should compute curvature report", async () => {
    const report = await api("GET", "/v1/bundles/sdk_test/curvature");
    assert.ok(report.K >= 0);
    assert.ok(report.confidence > 0);
    assert.ok(report.capacity > 0);
    assert.ok(Array.isArray(report.per_field));
    console.log(
      `  K=${report.K}, confidence=${report.confidence}, fields=${report.per_field.length}`
    );
  });

  it("should check consistency (Čech H¹)", async () => {
    const report = await api(
      "GET",
      "/v1/bundles/sdk_test/consistency"
    );
    assert.equal(report.h1, 0, "Should be fully consistent");
    console.log(`  H¹=${report.h1} — fully consistent`);
  });

  it("should aggregate (fiber integral)", async () => {
    const result = await api("POST", "/v1/bundles/sdk_test/aggregate", {
      group_by: "active",
      field: "score",
    });
    assert.ok(result.groups);
    const activeGroup = result.groups["true"];
    assert.ok(activeGroup, "Should have 'true' group");
    assert.equal(activeGroup.count, 4);
    console.log(
      `  Active: count=${activeGroup.count}, avg=${activeGroup.avg}`
    );
  });
});
