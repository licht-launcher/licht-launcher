import { vi } from "vitest";

/**
 * jsdom has no layout, so virtualized content tables render no rows. This gives
 * them a 260px viewport, 52px rows and a `scrollTo` that scrolls, and returns
 * the cleanup that restores jsdom.
 */
export function mockVirtualListLayout(): () => void {
  const originalScrollTo = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    "scrollTo",
  );
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      if (this.dataset.slot === "scroll-area-viewport") return 260;
      return this.tagName === "TR" && this.hasAttribute("data-index") ? 52 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(960);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return this.dataset.slot === "scroll-area-viewport" ? 260 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return [
        ...this.querySelectorAll<HTMLTableRowElement>("tbody > tr"),
      ].reduce(
        (total, row) =>
          total +
          (row.hasAttribute("data-index")
            ? row.offsetHeight
            : Number.parseFloat(row.cells[0]?.style.height || "0")),
        0,
      );
    },
  );
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: function (this: HTMLElement, options: ScrollToOptions) {
      const offset = Math.max(
        0,
        Math.min(
          options.top ?? this.scrollTop,
          this.scrollHeight - this.clientHeight,
        ),
      );
      if (offset !== this.scrollTop) {
        this.scrollTop = offset;
        queueMicrotask(() => this.dispatchEvent(new Event("scroll")));
      }
    },
  });
  return () => {
    vi.restoreAllMocks();
    if (originalScrollTo)
      Object.defineProperty(
        HTMLElement.prototype,
        "scrollTo",
        originalScrollTo,
      );
    else Reflect.deleteProperty(HTMLElement.prototype, "scrollTo");
  };
}
