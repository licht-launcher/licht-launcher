import { onTestFinished, vi } from "vitest";

/**
 * jsdom has no layout, so a virtualized list would render no rows. Gives the
 * scroll viewport and the list rows a size for the current test only.
 */
export function mockListLayout() {
  const height = vi
    .spyOn(HTMLElement.prototype, "offsetHeight", "get")
    .mockImplementation(function (this: HTMLElement) {
      if (this.dataset.slot === "scroll-area-viewport") return 600;
      return this.tagName === "LI" && this.hasAttribute("data-index") ? 72 : 0;
    });
  const width = vi
    .spyOn(HTMLElement.prototype, "offsetWidth", "get")
    .mockReturnValue(800);
  onTestFinished(() => {
    height.mockRestore();
    width.mockRestore();
  });
}
