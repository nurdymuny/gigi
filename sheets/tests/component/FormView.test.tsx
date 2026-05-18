import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { FormView } from "../../src/components/FormView";
import type { BundleSchema } from "../../src/lib/gigi-client";

const schema: BundleSchema = {
  name: "applicants",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [
    { name: "name", type: "text" },
    { name: "experience_years", type: "numeric" },
    { name: "active", type: "boolean" },
  ],
  indexed_fields: ["id"],
  records: 0,
  storage_mode: "mmap",
} as unknown as BundleSchema;

describe("FormView", () => {
  it("renders one input per non-opaque field", () => {
    render(<FormView schema={schema} onSubmit={() => undefined} />);
    expect(screen.getByTestId("form-field-id")).toBeInTheDocument();
    expect(screen.getByTestId("form-field-name")).toBeInTheDocument();
    expect(screen.getByTestId("form-field-experience_years")).toBeInTheDocument();
    expect(screen.getByTestId("form-field-active")).toBeInTheDocument();
  });

  it("parses values per field type on submit", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<FormView schema={schema} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByTestId("form-field-id"), {
      target: { value: "A-100" },
    });
    fireEvent.change(screen.getByTestId("form-field-name"), {
      target: { value: "Mira" },
    });
    fireEvent.change(screen.getByTestId("form-field-experience_years"), {
      target: { value: "6" },
    });
    fireEvent.change(screen.getByTestId("form-field-active"), {
      target: { value: "true" },
    });
    fireEvent.click(screen.getByTestId("form-view-submit"));
    await waitFor(() => expect(onSubmit).toHaveBeenCalled());
    expect(onSubmit.mock.calls[0][0]).toEqual({
      id: "A-100",
      name: "Mira",
      experience_years: 6,
      active: true,
    });
  });

  it("surfaces an upstream submit error in the error panel", async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error("engine rejected: dup key"));
    render(<FormView schema={schema} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByTestId("form-field-id"), {
      target: { value: "A-100" },
    });
    fireEvent.click(screen.getByTestId("form-view-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("form-view-error")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("form-view-error")).toHaveTextContent(/dup key/);
  });

  it("shows a confirmation after a successful submit + clears the form", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<FormView schema={schema} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByTestId("form-field-id"), {
      target: { value: "A-101" },
    });
    fireEvent.click(screen.getByTestId("form-view-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("form-view-confirmation")).toBeInTheDocument(),
    );
    expect(
      (screen.getByTestId("form-field-id") as HTMLInputElement).value,
    ).toBe("");
  });

  it("reset clears any partial input", () => {
    render(<FormView schema={schema} onSubmit={() => undefined} />);
    fireEvent.change(screen.getByTestId("form-field-id"), {
      target: { value: "A-999" },
    });
    fireEvent.click(screen.getByTestId("form-view-reset"));
    expect(
      (screen.getByTestId("form-field-id") as HTMLInputElement).value,
    ).toBe("");
  });

  it("renders an empty state when schema is null", () => {
    render(<FormView schema={null} onSubmit={() => undefined} />);
    expect(screen.getByTestId("form-view-empty")).toBeInTheDocument();
  });
});
