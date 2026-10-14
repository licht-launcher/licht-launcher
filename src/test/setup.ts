import type { TestingLibraryMatchers } from "@testing-library/jest-dom/matchers";
import * as matchers from "@testing-library/jest-dom/matchers";
import { cleanup } from "@testing-library/react";
import { afterEach, expect } from "vitest";

// The jest-dom Vitest adapter still declares the pre-Vitest 5 Assertion interface.
declare module "vitest" {
  interface Matchers<R, T> extends TestingLibraryMatchers<T, R> {}
}

expect.extend(matchers);

class ResizeObserverStub implements ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

globalThis.ResizeObserver = ResizeObserverStub;

Element.prototype.scrollIntoView = () => {};
window.scrollTo = () => {};

if (typeof HTMLElement.prototype.hasPointerCapture !== "function") {
  HTMLElement.prototype.hasPointerCapture = () => false;
}

if (typeof HTMLElement.prototype.setPointerCapture !== "function") {
  HTMLElement.prototype.setPointerCapture = () => {};
}

if (typeof HTMLElement.prototype.releasePointerCapture !== "function") {
  HTMLElement.prototype.releasePointerCapture = () => {};
}

afterEach(() => {
  cleanup();
});
