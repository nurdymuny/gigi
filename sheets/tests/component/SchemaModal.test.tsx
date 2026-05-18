import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SchemaModal } from "../../src/components/SchemaModal";
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
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id", "site_id"],
  records: 1284,
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

describe("SchemaModal — list view", () => {
  it("renders nothing when closed", () => {
    render(
      <SchemaModal
        open={false}
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    expect(screen.queryByTestId("schema-modal")).toBeNull();
  });

  it("lists every base + fiber field with the right tags", () => {
    render(
      <SchemaModal
        open
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    // Each field has a row
    expect(screen.getByTestId("schema-field-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("schema-field-site_id")).toBeInTheDocument();
    expect(screen.getByTestId("schema-field-temp")).toBeInTheDocument();
    expect(screen.getByTestId("schema-field-operator")).toBeInTheDocument();

    // Primary key tagged + drop button disabled
    expect(screen.getByTestId("schema-drop-sensor_id")).toBeDisabled();
    // Indexed field tagged
    expect(screen.getByTestId("schema-field-site_id")).toHaveTextContent("indexed");
    // Encrypted field tagged
    expect(screen.getByTestId("schema-field-operator")).toHaveTextContent(
      "encrypted · indexed",
    );
  });

  it("clicking Drop sends POST /drop-field and calls onMutated", async () => {
    // window.confirm → always yes for the test
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const fetcher = vi.fn().mockResolvedValue(jsonResponse({ status: "ok" }));
    const onMutated = vi.fn();
    render(
      <SchemaModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={onMutated}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-drop-temp"));
    await waitFor(() => expect(onMutated).toHaveBeenCalled());
    const [url, init] = fetcher.mock.calls[0];
    expect(url).toContain("/drop-field");
    expect(JSON.parse(init.body)).toEqual({ field: "temp" });
  });

  it("cancelling the confirm prompt does NOT contact the engine", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const fetcher = vi.fn();
    render(
      <SchemaModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-drop-temp"));
    expect(fetcher).not.toHaveBeenCalled();
  });
});

describe("SchemaModal — add field flow", () => {
  it("switches to the form when '+ Add field' is clicked", () => {
    render(
      <SchemaModal
        open
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-add"));
    expect(screen.getByTestId("schema-add-form")).toBeInTheDocument();
  });

  it("submits name + type to /add-field", async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse({ status: "ok" }));
    const onMutated = vi.fn();
    render(
      <SchemaModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={onMutated}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-add"));
    fireEvent.change(screen.getByTestId("schema-form-name"), {
      target: { value: "pressure_hpa" },
    });
    fireEvent.change(screen.getByTestId("schema-form-type"), {
      target: { value: "numeric" },
    });
    fireEvent.click(screen.getByTestId("schema-form-submit"));
    await waitFor(() => expect(onMutated).toHaveBeenCalled());
    const body = JSON.parse(fetcher.mock.calls[0][1].body);
    expect(body).toMatchObject({ name: "pressure_hpa", type: "numeric" });
  });

  it("shows an inline error when the field name is invalid", async () => {
    const fetcher = vi.fn();
    render(
      <SchemaModal
        open
        client={makeClient(fetcher)}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-add"));
    fireEvent.change(screen.getByTestId("schema-form-name"), {
      target: { value: "bad name with spaces" },
    });
    fireEvent.click(screen.getByTestId("schema-form-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("schema-modal-error")).toBeInTheDocument(),
    );
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("warns that the encryption mode is a demo overlay, not active client-side crypto", () => {
    // Pre-launch security audit (2026-05-18): users were at risk of
    // believing checking 'opaque' here would actually encrypt their
    // data client-side. The note now flags it as a display-only
    // overlay — real crypto is enforced engine-side — so PHI doesn't
    // get loaded under a false sense of safety.
    render(
      <SchemaModal
        open
        client={makeClient(vi.fn())}
        bundle="sensors"
        schema={SCHEMA}
        onClose={() => {}}
        onMutated={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("schema-add"));
    fireEvent.change(screen.getByTestId("schema-form-encryption"), {
      target: { value: "opaque" },
    });
    const note = screen.getByTestId("schema-add-form");
    expect(note).toHaveTextContent(/demo overlay/i);
    expect(note).toHaveTextContent(/engine/i);
  });
});
