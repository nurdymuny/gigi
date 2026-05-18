import { describe, expect, it, vi } from "vitest";
import { applyWorkflow } from "../../src/lib/apply-workflow";
import type { SheetsClient } from "../../src/lib/gigi-client";
import { findWorkflowTemplate } from "../../src/lib/workflow-templates";

function makeClient() {
  const createBundle = vi.fn().mockResolvedValue(undefined);
  const insert = vi.fn().mockResolvedValue(undefined);
  return {
    createBundle,
    insert,
  } as unknown as SheetsClient & {
    createBundle: ReturnType<typeof vi.fn>;
    insert: ReturnType<typeof vi.fn>;
  };
}

describe("applyWorkflow", () => {
  it("creates one bundle per template.bundles entry and inserts every row", async () => {
    const tpl = findWorkflowTemplate("project_tracker")!;
    const client = makeClient();
    const result = await applyWorkflow(tpl, client);
    expect(client.createBundle).toHaveBeenCalledTimes(1);
    expect(client.createBundle.mock.calls[0][0]).toMatchObject({
      name: "workflow_projects",
    });
    // Insert called at least once with the seed rows.
    expect(client.insert).toHaveBeenCalled();
    expect(result.defaultBundle).toBe("workflow_projects");
    expect(result.rowCount).toBeGreaterThan(0);
  });

  it("creates both bundles for the CRM (two-bundle) template", async () => {
    const tpl = findWorkflowTemplate("crm")!;
    const client = makeClient();
    const result = await applyWorkflow(tpl, client);
    expect(client.createBundle).toHaveBeenCalledTimes(2);
    const created = client.createBundle.mock.calls.map((c) => c[0].name);
    expect(created).toContain("workflow_crm_contacts");
    expect(created).toContain("workflow_crm_deals");
    expect(result.defaultBundle).toBe("workflow_crm_deals");
  });

  it("calls onProgress with bundle name, done, total", async () => {
    const tpl = findWorkflowTemplate("recruiting")!;
    const client = makeClient();
    const progress: Array<{ bundleName: string; done: number; total: number }> = [];
    await applyWorkflow(tpl, client, (p) =>
      progress.push({ bundleName: p.bundleName, done: p.done, total: p.total }),
    );
    expect(progress.length).toBeGreaterThan(0);
    const last = progress[progress.length - 1];
    expect(last.bundleName).toBe("workflow_recruiting");
    expect(last.done).toBe(last.total);
  });

  it("propagates engine errors instead of swallowing them", async () => {
    const tpl = findWorkflowTemplate("inventory")!;
    const client = makeClient();
    client.createBundle.mockRejectedValueOnce(new Error("conflict: bundle exists"));
    await expect(applyWorkflow(tpl, client)).rejects.toThrow(/conflict/);
  });
});
