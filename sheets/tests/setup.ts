import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
});

/**
 * jsdom does not implement layout. @tanstack/react-virtual reads
 * clientWidth/clientHeight from the scroll container and uses
 * getBoundingClientRect for measurement. Stub everything it touches.
 */

const VIEWPORT_W = 1024;
const VIEWPORT_H = 768;

Object.defineProperty(window.HTMLElement.prototype, "getBoundingClientRect", {
  configurable: true,
  value() {
    return {
      x: 0,
      y: 0,
      width: VIEWPORT_W,
      height: VIEWPORT_H,
      top: 0,
      left: 0,
      right: VIEWPORT_W,
      bottom: VIEWPORT_H,
      toJSON: () => "",
    };
  },
});

Object.defineProperty(window.HTMLElement.prototype, "clientWidth", {
  configurable: true,
  get() {
    return VIEWPORT_W;
  },
});

Object.defineProperty(window.HTMLElement.prototype, "clientHeight", {
  configurable: true,
  get() {
    return VIEWPORT_H;
  },
});

Object.defineProperty(window.HTMLElement.prototype, "offsetWidth", {
  configurable: true,
  get() {
    return VIEWPORT_W;
  },
});

Object.defineProperty(window.HTMLElement.prototype, "offsetHeight", {
  configurable: true,
  get() {
    return VIEWPORT_H;
  },
});

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
