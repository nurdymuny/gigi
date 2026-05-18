import { describe, expect, it } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useEditHistory } from "../../src/lib/use-edit-history";

describe("useEditHistory", () => {
  it("starts empty — undo/redo are no-ops", () => {
    const { result } = renderHook(() => useEditHistory(50));
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
    expect(result.current.undo()).toBeNull();
    expect(result.current.redo()).toBeNull();
  });

  it("push() records edits and enables undo", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 30 });
    });
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it("undo() returns the most-recent edit reversed (after→before)", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 30 });
    });
    let undone: ReturnType<typeof result.current.undo> | undefined;
    act(() => {
      undone = result.current.undo();
    });
    expect(undone).toEqual({ kind: "cell", rowKey: "S-1", field: "temp", value: 22 });
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(true);
  });

  it("redo() restores the last undone edit (forward direction)", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 30 });
      result.current.undo();
    });
    let redone: ReturnType<typeof result.current.redo> | undefined;
    act(() => {
      redone = result.current.redo();
    });
    expect(redone).toEqual({ kind: "cell", rowKey: "S-1", field: "temp", value: 30 });
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it("pushing a new edit clears the redo stack", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 30 });
      result.current.undo();
    });
    expect(result.current.canRedo).toBe(true);
    act(() => {
      result.current.push({ rowKey: "S-1", field: "humidity", before: 60, after: 70 });
    });
    expect(result.current.canRedo).toBe(false);
  });

  it("undoes in reverse-chronological order across multiple edits", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 23 });
      result.current.push({ rowKey: "S-1", field: "temp", before: 23, after: 24 });
      result.current.push({ rowKey: "S-1", field: "temp", before: 24, after: 25 });
    });
    let u1: ReturnType<typeof result.current.undo> | undefined;
    let u2: ReturnType<typeof result.current.undo> | undefined;
    act(() => {
      u1 = result.current.undo();
      u2 = result.current.undo();
    });
    expect(u1).toEqual({ kind: "cell", rowKey: "S-1", field: "temp", value: 24 });
    expect(u2).toEqual({ kind: "cell", rowKey: "S-1", field: "temp", value: 23 });
  });

  it("caps the history at maxSize (oldest evicted)", () => {
    const { result } = renderHook(() => useEditHistory(3));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "a", before: 1, after: 2 });
      result.current.push({ rowKey: "S-1", field: "b", before: 3, after: 4 });
      result.current.push({ rowKey: "S-1", field: "c", before: 5, after: 6 });
      result.current.push({ rowKey: "S-1", field: "d", before: 7, after: 8 });
    });
    // Now we should only be able to undo back 3 edits, not 4.
    let count = 0;
    act(() => {
      while (result.current.undo() !== null) count += 1;
    });
    expect(count).toBe(3);
  });

  it("undo of a row delete returns a 'restore' op with the full row payload", () => {
    const { result } = renderHook(() => useEditHistory(50));
    const row = { sensor_id: "S-1", temp: 22, humidity: 60 };
    act(() => {
      result.current.push({ kind: "delete", rowKey: "S-1", row });
    });
    let undone: ReturnType<typeof result.current.undo> | undefined;
    act(() => {
      undone = result.current.undo();
    });
    expect(undone).toEqual({ kind: "restore", rowKey: "S-1", row });
    expect(result.current.canRedo).toBe(true);
  });

  it("redo of an undone delete returns a 'delete' op (re-removes the row)", () => {
    const { result } = renderHook(() => useEditHistory(50));
    const row = { sensor_id: "S-1", temp: 22 };
    act(() => {
      result.current.push({ kind: "delete", rowKey: "S-1", row });
      result.current.undo();
    });
    let redone: ReturnType<typeof result.current.redo> | undefined;
    act(() => {
      redone = result.current.redo();
    });
    expect(redone).toEqual({ kind: "delete", rowKey: "S-1" });
  });

  it("undo of an insert returns a 'delete' op (removes the inserted row)", () => {
    const { result } = renderHook(() => useEditHistory(50));
    const row = { sensor_id: "S-9", temp: 99 };
    act(() => {
      result.current.push({ kind: "insert", rowKey: "S-9", row });
    });
    let undone: ReturnType<typeof result.current.undo> | undefined;
    act(() => {
      undone = result.current.undo();
    });
    expect(undone).toEqual({ kind: "delete", rowKey: "S-9" });
  });

  it("mixed timeline: cell edit → delete → undo → undo restores order", () => {
    const { result } = renderHook(() => useEditHistory(50));
    const row = { sensor_id: "S-1", temp: 30 };
    act(() => {
      result.current.push({ rowKey: "S-1", field: "temp", before: 22, after: 30 });
      result.current.push({ kind: "delete", rowKey: "S-1", row });
    });
    let u1: ReturnType<typeof result.current.undo> | undefined;
    let u2: ReturnType<typeof result.current.undo> | undefined;
    act(() => {
      u1 = result.current.undo(); // undoes the delete → restore
      u2 = result.current.undo(); // undoes the cell edit → cell op
    });
    expect(u1).toEqual({ kind: "restore", rowKey: "S-1", row });
    expect(u2).toEqual({ kind: "cell", rowKey: "S-1", field: "temp", value: 22 });
  });

  it("clear() wipes both stacks", () => {
    const { result } = renderHook(() => useEditHistory(50));
    act(() => {
      result.current.push({ rowKey: "S-1", field: "x", before: 1, after: 2 });
      result.current.undo();
    });
    expect(result.current.canRedo).toBe(true);
    act(() => result.current.clear());
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });
});
