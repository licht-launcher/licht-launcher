import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";

import { ModpackContentCard } from "./modpack-content-card";

const getModpack = vi.hoisted(() => vi.fn());
const listModpackVersions = vi.hoisted(() => vi.fn());

vi.mock("@/api/commands", () => ({
  getModpack,
  listModpackVersions,
  listModpackSources: vi.fn(),
  searchModpacks: vi.fn(),
}));

const STORE: ModpackOrigin = {
  type: "modpack",
  source: "modrinth",
  projectId: "fo",
  versionId: "v15",
  iconUrl: null,
};

function entry(name: string, kind: InstanceContentKind): InstanceContentEntry {
  return {
    name,
    kind,
    enabled: true,
    size: 1,
    title: name,
    version: null,
    iconUrl: null,
    fromModpack: true,
  };
}

const ENTRIES = [
  entry("a.jar", "mods"),
  entry("b.jar", "mods"),
  entry("pack.zip", "resourcePacks"),
];

function renderCard(
  origin: ModpackOrigin,
  entries: InstanceContentEntry[] = ENTRIES,
  viewing = false,
) {
  const onViewingChange = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ModpackContentCard
        origin={origin}
        entries={entries}
        viewing={viewing}
        onViewingChange={onViewingChange}
      />
    </QueryClientProvider>,
  );
  return { onViewingChange };
}

describe("ModpackContentCard", () => {
  beforeEach(() => {
    getModpack.mockReset();
    listModpackVersions.mockReset();
  });

  it("shows what the modpack ships and which version provides it", async () => {
    getModpack.mockResolvedValue({
      name: "Fabulously Optimized",
      iconUrl: null,
    });
    listModpackVersions.mockResolvedValue([
      { id: "v14", name: "14.0.0", publishedAt: null },
      { id: "v15", name: "15.0.0-beta.3", publishedAt: "2020-01-01T00:00:00Z" },
    ]);

    renderCard(STORE);

    expect(screen.getByText("2 mods")).toBeInTheDocument();
    expect(screen.getByText("1 resource pack")).toBeInTheDocument();
    expect(await screen.findByText("Fabulously Optimized")).toBeInTheDocument();
    expect(await screen.findByText("15.0.0-beta.3")).toBeInTheDocument();
    expect(screen.getByText(/^Updated /)).toBeInTheDocument();
    expect(getModpack).toHaveBeenCalledWith("modrinth", "fo");
  });

  it("says when the modpack details cannot be loaded", async () => {
    getModpack.mockRejectedValue(new Error("offline"));
    listModpackVersions.mockRejectedValue(new Error("offline"));

    renderCard(STORE);

    expect(
      await screen.findByText("Modpack details are unavailable."),
    ).toBeInTheDocument();
    expect(screen.getByText("2 mods")).toBeInTheDocument();
  });

  it("shows a pack imported from disk without asking a store", () => {
    renderCard({ type: "modpackFile", source: "modrinth" });

    expect(
      screen.getByText("Imported from a modpack file"),
    ).toBeInTheDocument();
    expect(getModpack).not.toHaveBeenCalled();
  });

  it("toggles between viewing and hiding the modpack content", () => {
    const { onViewingChange } = renderCard({
      type: "modpackFile",
      source: "modrinth",
    });

    const button = screen.getByRole("button", { name: "View content" });
    expect(button).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(button);
    expect(onViewingChange).toHaveBeenCalledWith(true);
  });

  it("cannot view content when the modpack ships none", () => {
    renderCard({ type: "modpackFile", source: "modrinth" }, []);

    expect(screen.getByRole("button", { name: "View content" })).toBeDisabled();
  });
});
