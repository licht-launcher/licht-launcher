import type { TestingLibraryMatchers } from "@testing-library/jest-dom/matchers";
import * as matchers from "@testing-library/jest-dom/matchers";
import { cleanup } from "@testing-library/react";
import "sonner";
import { afterEach, expect } from "vitest";

// The jest-dom Vitest adapter still declares the pre-Vitest 5 Assertion interface.
declare module "vitest" {
  interface Matchers<R, T> extends TestingLibraryMatchers<T, R> {}
}

expect.extend(matchers);

// Sonner injects its stylesheet on import, unlike the app CSS that Vitest skips.
// jsdom matches every rule against an element and its ancestors on each
// getComputedStyle call, which made role queries in full-app tests CPU-bound.
for (const style of document.head.querySelectorAll("style")) {
  if (style.textContent?.includes("[data-sonner-toaster]")) style.remove();
}

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
