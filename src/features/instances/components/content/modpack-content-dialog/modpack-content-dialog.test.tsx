import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

import { ModpackContentDialog } from "./modpack-content-dialog";

function entry(title: string): InstanceContentEntry {
  return {
    name: `${title.toLowerCase()}.jar`,
    kind: "mods",
    enabled: true,
    size: 1,
    title,
    version: null,
    iconUrl: null,
    fromModpack: true,
  };
}

function renderDialog(open: boolean) {
  render(
    <ModpackContentDialog
      open={open}
      onOpenChange={vi.fn()}
      folder="classic"
      entries={[entry("Sodium"), entry("Iris")]}
    />,
  );
}

describe("ModpackContentDialog", () => {
  // jsdom has no layout, so give the virtualized list a viewport and row heights.
  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        if (this.dataset.slot === "scroll-area-viewport") return 260;
        return this.tagName === "TR" ? 52 : 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(960);
  });

  afterEach(() => vi.restoreAllMocks());

  it("lists the modpack content by name", () => {
    renderDialog(true);

    const dialog = screen.getByRole("dialog", { name: "Modpack content" });
    const titles = within(dialog)
      .getAllByRole("row")
      .map((row) => row.textContent);
    expect(titles[0]).toContain("Iris");
    expect(titles[1]).toContain("Sodium");
  });

  it("cannot disable or remove content the modpack manages", () => {
    renderDialog(true);

    const dialog = screen.getByRole("dialog", { name: "Modpack content" });
    expect(within(dialog).getByText("Sodium")).toBeInTheDocument();
    expect(within(dialog).queryByRole("switch")).not.toBeInTheDocument();
    expect(
      within(dialog).queryByRole("button", { name: /^Remove / }),
    ).not.toBeInTheDocument();
  });

  it("renders nothing while closed", () => {
    renderDialog(false);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
