import { describe, expect, it } from "vitest";
import { parseCsv } from "../../src/lib/csv";
import {
  findWorkflowTemplate,
  workflowCsv,
  WORKFLOW_TEMPLATES,
} from "../../src/lib/workflow-templates";

describe("workflow-templates · structural integrity", () => {
  it("ships exactly six templates", () => {
    expect(WORKFLOW_TEMPLATES).toHaveLength(6);
  });

  it("each template has a unique id", () => {
    const ids = WORKFLOW_TEMPLATES.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("each template's defaultBundle exists in its bundles list", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      const names = t.bundles.map((b) => b.name);
      expect(names, `${t.id} defaultBundle`).toContain(t.defaultBundle);
    }
  });

  it("each template has at least one bundle", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      expect(t.bundles.length, `${t.id} bundle count`).toBeGreaterThan(0);
    }
  });

  it("each bundle has a primary key listed in its fields", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      for (const b of t.bundles) {
        for (const k of b.keys) {
          expect(Object.keys(b.fields), `${t.id}.${b.name} key ${k}`).toContain(
            k,
          );
        }
      }
    }
  });

  it("each bundle's suggestedCover exists in its fields", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      for (const b of t.bundles) {
        expect(
          Object.keys(b.fields),
          `${t.id}.${b.name} suggestedCover`,
        ).toContain(b.suggestedCover);
      }
    }
  });

  it("each template's defaultView is one of the supported views", () => {
    const supported = new Set([
      "grid",
      "kanban",
      "gallery",
      "form",
      "calendar",
      "gql",
    ]);
    for (const t of WORKFLOW_TEMPLATES) {
      expect(supported.has(t.defaultView), `${t.id} defaultView`).toBe(true);
    }
  });
});

describe("workflow-templates · seed CSV correctness", () => {
  it("each seedCsv parses into rows with column count = fields count", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      for (const b of t.bundles) {
        const csv = workflowCsv(b);
        const parsed = parseCsv(csv);
        const expectedColumns = Object.keys(b.fields);
        expect(parsed.headers, `${t.id}.${b.name} headers`).toEqual(
          expectedColumns,
        );
        expect(parsed.rows.length, `${t.id}.${b.name} row count`).toBeGreaterThan(
          0,
        );
        // Every row should have a value for the primary key.
        for (const r of parsed.rows) {
          for (const k of b.keys) {
            expect(r[k], `${t.id}.${b.name} row key ${k}`).toBeTruthy();
          }
        }
      }
    }
  });

  it("primary keys are unique within each bundle", () => {
    for (const t of WORKFLOW_TEMPLATES) {
      for (const b of t.bundles) {
        const parsed = parseCsv(workflowCsv(b));
        const keyField = b.keys[0];
        const keys = parsed.rows.map((r) => String(r[keyField]));
        expect(new Set(keys).size, `${t.id}.${b.name} unique keys`).toBe(
          keys.length,
        );
      }
    }
  });
});

describe("workflow-templates · findWorkflowTemplate", () => {
  it("looks up by id", () => {
    expect(findWorkflowTemplate("project_tracker")?.title).toBe(
      "Project tracker",
    );
    expect(findWorkflowTemplate("crm")?.bundles.length).toBe(2);
  });

  it("returns null for unknown ids", () => {
    expect(findWorkflowTemplate("nope")).toBeNull();
  });
});
