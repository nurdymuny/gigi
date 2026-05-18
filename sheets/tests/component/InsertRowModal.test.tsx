import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { InsertRowModal } from "../../src/components/InsertRowModal";
import {
  SheetsClient,
  type BundleSchema,
  type Fetcher,
} from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "active", type: "boolean" },
  ],
  indexed_fields: ["sensor_id"],
  records: 0,
  storage_mode: "mmap",
};

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function makeClient(fetcher: ReturnType<typeof vi.fn>) {
  return new SheetsClient({
    baseUrl: "http://localhost:3142",
    fetch: fetcher as unknown as Fetcher,
  });
}

describe("InsertRowModal", () => {
  it("renders nothing when closed", () => {
    render(
      <InsertRowModal
        open={false}
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onInserted={() => {}}
      />,
    );
    expect(screen.queryByTestId("insert-row-modal")).toBeNull();
  });

  it("renders one labeled input per field, with key tagged", () => {
    render(
      <InsertRowModal
        open
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onInserted={() => {}}
      />,
    );
    expect(screen.getByTestId("insert-row-input-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("insert-row-input-site_id")).toBeInTheDocument();
    expect(screen.getByTestId("insert-row-input-temp")).toHaveAttribute("type", "number");
    // Boolean field renders as a select.
    expect(screen.getByTestId("insert-row-input-active").tagName).toBe("SELECT");
    // Key field carries the "key" tag.
    expect(screen.getByTestId("insert-row-field-sensor_id")).toHaveTextContent("key");
  });

  it("validates that the key field is required", async () => {
    const fetcher = vi.fn();
    render(
      <InsertRowModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onInserted={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("insert-row-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("insert-row-error")).toBeInTheDocument(),
    );
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("submits, coercing numeric and boolean values to their typed forms", async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse({ status: "ok" }));
    const onInserted = vi.fn();
    render(
      <InsertRowModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onInserted={onInserted}
      />,
    );
    fireEvent.change(screen.getByTestId("insert-row-input-sensor_id"), {
      target: { value: "S-007" },
    });
    fireEvent.change(screen.getByTestId("insert-row-input-site_id"), {
      target: { value: "North" },
    });
    fireEvent.change(screen.getByTestId("insert-row-input-temp"), {
      target: { value: "23.5" },
    });
    fireEvent.change(screen.getByTestId("insert-row-input-active"), {
      target: { value: "true" },
    });
    fireEvent.click(screen.getByTestId("insert-row-submit"));
    await waitFor(() => expect(onInserted).toHaveBeenCalledWith("S-007"));

    const body = JSON.parse(fetcher.mock.calls[0][1].body);
    expect(body.records[0]).toEqual({
      sensor_id: "S-007",
      site_id: "North",
      temp: 23.5,
      active: true,
    });
  });

  it("renders empty values as null (not as the empty string)", async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse({ status: "ok" }));
    render(
      <InsertRowModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onInserted={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("insert-row-input-sensor_id"), {
      target: { value: "S-008" },
    });
    // leave site_id, temp, active blank
    fireEvent.click(screen.getByTestId("insert-row-submit"));
    await waitFor(() => expect(fetcher).toHaveBeenCalled());
    const body = JSON.parse(fetcher.mock.calls[0][1].body);
    expect(body.records[0]).toEqual({
      sensor_id: "S-008",
      site_id: null,
      temp: null,
      active: null,
    });
  });

  it("surfaces an engine error inline and does NOT close", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValue(new Response("conflict", { status: 409 }));
    const onClose = vi.fn();
    const onInserted = vi.fn();
    render(
      <InsertRowModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={onClose}
        onInserted={onInserted}
      />,
    );
    fireEvent.change(screen.getByTestId("insert-row-input-sensor_id"), {
      target: { value: "S-DUP" },
    });
    fireEvent.click(screen.getByTestId("insert-row-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("insert-row-error")).toBeInTheDocument(),
    );
    expect(onInserted).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });
});
