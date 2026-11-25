import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { mockVirtualListLayout } from "@/features/instances/tests/support/virtual-list-layout";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

import { ModpackContentDialog } from "./modpack-content-dialog";

const getModpack = vi.hoisted(() => vi.fn());
const listModpackVersions = vi.hoisted(() => vi.fn());

vi.mock("@/api/commands", () => ({
  getModpack,
  listModpackVersions,
  listModpackSources: vi.fn(),
  searchModpacks: vi.fn(),
}));

const IMPORTED: ModpackOrigin = { type: "modpackFile", source: "modrinth" };

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

function renderDialog(
  open = true,
  origin: ModpackOrigin = IMPORTED,
  linked: LinkedModpack | null = null,
) {
  const onToggle = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ModpackContentDialog
        open={open}
        onOpenChange={vi.fn()}
        origin={origin}
        linked={linked}
        iconPath={null}
        folder="classic"
        entries={ENTRIES}
        actions={{ disabled: false, onToggle }}
      />
    </QueryClientProvider>,
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
  let restoreLayout: () => void = () => {};
  beforeEach(() => {
    restoreLayout = mockVirtualListLayout();
  });

  afterEach(() => restoreLayout());

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

  it("filters by kind and search", async () => {
    renderDialog();

    fireEvent.click(screen.getByRole("combobox", { name: "Filter content" }));
    fireEvent.click(await screen.findByRole("option", { name: "Mods" }));
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

  it("describes the pack, its version, provider and release age", async () => {
    getModpack.mockResolvedValue({
      name: "Fabulously Optimized",
      author: "robotkoer",
      iconUrl: null,
    });
    listModpackVersions.mockResolvedValue([
      { id: "v15", name: "15.0.0-beta.3", publishedAt: "2020-01-01T00:00:00Z" },
    ]);

    renderDialog(true, {
      type: "modpack",
      source: "modrinth",
      projectId: "fo",
      versionId: "v15",
      iconUrl: null,
    });

    expect(
      await screen.findByText(
        /^Fabulously Optimized 15\.0\.0-beta\.3 · by robotkoer · Modrinth · Updated .+ ago$/,
      ),
    ).toBeInTheDocument();
  });

  it("describes a pack from the details saved at install, without asking the store", () => {
    renderDialog(
      true,
      {
        type: "modpack",
        source: "curseforge",
        projectId: "123",
        versionId: "456",
        iconUrl: null,
      },
      {
        name: "All the Mods",
        author: null,
        versionName: "ATM 1.0",
        publishedAt: null,
      },
    );

    expect(
      screen.getByRole("dialog", { name: "Modpack content" }),
    ).toHaveAccessibleDescription("All the Mods ATM 1.0 · CurseForge");
    expect(getModpack).not.toHaveBeenCalled();
  });

  it("describes an imported pack from what its archive declared", () => {
    renderDialog(true, IMPORTED, {
      name: "All the Mods",
      author: "ATM Team",
      versionName: "1.0",
      publishedAt: null,
    });

    expect(
      screen.getByRole("dialog", { name: "Modpack content" }),
    ).toHaveAccessibleDescription(
      "All the Mods 1.0 · by ATM Team · Installed from a Modrinth file",
    );
  });

  it("says where an imported pack came from", () => {
    renderDialog();

    expect(
      screen.getByRole("dialog", { name: "Modpack content" }),
    ).toHaveAccessibleDescription(
      "Imported modpack · Installed from a Modrinth file",
    );
    expect(getModpack).not.toHaveBeenCalled();
  });

  it("renders nothing while closed", () => {
    renderDialog(false);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
