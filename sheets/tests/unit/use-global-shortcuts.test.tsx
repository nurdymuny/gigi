import { describe, expect, it, vi } from "vitest";
import { render, fireEvent } from "@testing-library/react";
import { useGlobalShortcuts, type ShortcutBinding } from "../../src/lib/use-global-shortcuts";

function Harness({ bindings }: { bindings: ShortcutBinding[] }) {
  useGlobalShortcuts(bindings);
  return (
    <div>
      <input data-testid="text-input" />
      <textarea data-testid="text-area" />
      <div contentEditable data-testid="contenteditable" />
    </div>
  );
}

describe("useGlobalShortcuts", () => {
  it("fires the handler when the matching key is pressed on window", () => {
    const onFire = vi.fn();
    render(<Harness bindings={[{ key: "f", meta: true, handler: onFire }]} />);
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(onFire).toHaveBeenCalledOnce();
  });

  it("treats metaKey OR ctrlKey as 'meta' (cross-platform)", () => {
    const onFire = vi.fn();
    render(<Harness bindings={[{ key: "f", meta: true, handler: onFire }]} />);
    fireEvent.keyDown(window, { key: "f", metaKey: true });
    expect(onFire).toHaveBeenCalledOnce();
  });

  it("does NOT fire when modifier doesn't match", () => {
    const onFire = vi.fn();
    render(<Harness bindings={[{ key: "f", meta: true, handler: onFire }]} />);
    fireEvent.keyDown(window, { key: "f" });
    expect(onFire).not.toHaveBeenCalled();
  });

  it("respects shift modifier", () => {
    const onFire = vi.fn();
    render(<Harness bindings={[{ key: "a", meta: true, shift: true, handler: onFire }]} />);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(onFire).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true, shiftKey: true });
    expect(onFire).toHaveBeenCalledOnce();
  });

  it("does NOT fire when focus is in an <input> (typing should not trigger)", () => {
    const onFire = vi.fn();
    const { getByTestId } = render(
      <Harness bindings={[{ key: "/", handler: onFire }]} />,
    );
    const input = getByTestId("text-input");
    input.focus();
    fireEvent.keyDown(input, { key: "/" });
    expect(onFire).not.toHaveBeenCalled();
  });

  it("does NOT fire when focus is in a <textarea>", () => {
    const onFire = vi.fn();
    const { getByTestId } = render(
      <Harness bindings={[{ key: "/", handler: onFire }]} />,
    );
    const ta = getByTestId("text-area");
    ta.focus();
    fireEvent.keyDown(ta, { key: "/" });
    expect(onFire).not.toHaveBeenCalled();
  });

  it("DOES fire when binding has allowInInput: true even from an input", () => {
    const onFire = vi.fn();
    const { getByTestId } = render(
      <Harness
        bindings={[{ key: "Escape", allowInInput: true, handler: onFire }]}
      />,
    );
    const input = getByTestId("text-input");
    input.focus();
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onFire).toHaveBeenCalledOnce();
  });

  it("calls preventDefault when binding requests it", () => {
    const onFire = vi.fn();
    render(
      <Harness
        bindings={[{ key: "f", meta: true, preventDefault: true, handler: onFire }]}
      />,
    );
    const event = new KeyboardEvent("keydown", {
      key: "f",
      ctrlKey: true,
      cancelable: true,
    });
    const prevented = !window.dispatchEvent(event);
    expect(onFire).toHaveBeenCalledOnce();
    expect(prevented).toBe(true);
  });

  it("removes its listener on unmount", () => {
    const onFire = vi.fn();
    const { unmount } = render(
      <Harness bindings={[{ key: "f", meta: true, handler: onFire }]} />,
    );
    unmount();
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(onFire).not.toHaveBeenCalled();
  });

  it("matches key case-insensitively for letters", () => {
    const onFire = vi.fn();
    render(<Harness bindings={[{ key: "F", meta: true, handler: onFire }]} />);
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(onFire).toHaveBeenCalledOnce();
  });

  it("fires the first matching binding when multiple are registered", () => {
    const onFireA = vi.fn();
    const onFireB = vi.fn();
    render(
      <Harness
        bindings={[
          { key: "f", meta: true, handler: onFireA },
          { key: "g", meta: true, handler: onFireB },
        ]}
      />,
    );
    fireEvent.keyDown(window, { key: "g", ctrlKey: true });
    expect(onFireA).not.toHaveBeenCalled();
    expect(onFireB).toHaveBeenCalledOnce();
  });
});
