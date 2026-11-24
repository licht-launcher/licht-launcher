import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { ModpackContentDialog } from "./modpack-content-dialog";

function entry(
  title: string,
  kind: InstanceContentKind = "mods",
): InstanceContentEntry {
  return {
    name: `${title.toLowerCase()}.jar`,
    kind,
    enabled: true,
    size: 1,
    title,
    version: null,
    iconUrl: null,
    fromModpack: true,
  };
}

const ENTRIES = [
  entry("Sodium"),
  entry("Iris"),
  entry("Fresh", "resourcePacks"),
];

function renderDialog(open = true) {
  const onToggle = vi.fn();
  render(
    <ModpackContentDialog
      open={open}
      onOpenChange={vi.fn()}
      iconUrl={null}
      folder="classic"
      entries={ENTRIES}
      actions={{ disabled: false, onToggle }}
    />,
  );
  return { onToggle };
}

function listedTitles(): (string | null)[] {
  const dialog = screen.getByRole("dialog", { name: "Modpack content" });
  return within(dialog)
    .getAllByRole("row")
    .filter((row) => row.hasAttribute("data-index"))
    .map((row) => within(row).getAllByRole("paragraph")[0].textContent);
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

  it("lists the modpack content by name with counts per kind", () => {
    renderDialog();

    expect(listedTitles()).toEqual(["Fresh", "Iris", "Sodium"]);
    expect(screen.getByText("2 mods")).toBeInTheDocument();
    expect(screen.getByText("1 resource pack")).toBeInTheDocument();
  });

  it("can disable modpack content but not remove it", () => {
    const { onToggle } = renderDialog();

    fireEvent.click(screen.getByRole("switch", { name: "Disable Sodium" }));

    expect(onToggle).toHaveBeenCalledWith(
      expect.objectContaining({ title: "Sodium" }),
      false,
    );
    expect(
      screen.queryByRole("button", { name: /^Remove / }),
    ).not.toBeInTheDocument();
  });

  it("filters by kind and search", () => {
    renderDialog();
    const filters = screen.getByRole("group", { name: "Filter by type" });
    expect(
      within(filters)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["All", "Mods", "Resource Packs"]);

    fireEvent.click(within(filters).getByRole("button", { name: "Mods" }));
    expect(listedTitles()).toEqual(["Iris", "Sodium"]);

    fireEvent.change(
      screen.getByRole("textbox", { name: "Search modpack content" }),
      { target: { value: "sod" } },
    );
    expect(listedTitles()).toEqual(["Sodium"]);
  });

  it("reverses the order from the column header", () => {
    renderDialog();

    fireEvent.click(screen.getByRole("button", { name: /Content/ }));

    expect(listedTitles()).toEqual(["Sodium", "Iris", "Fresh"]);
  });

  it("renders nothing while closed", () => {
    renderDialog(false);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
