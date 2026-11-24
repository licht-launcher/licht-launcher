import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  linked = false,
) {
  const onView = vi.fn();
  const onOpenModpack = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ModpackContentCard
        origin={origin}
        entries={entries}
        onView={onView}
        onOpenModpack={linked ? onOpenModpack : undefined}
      />
    </QueryClientProvider>,
  );
  return { onView, onOpenModpack };
}

function mockStore() {
  getModpack.mockResolvedValue({
    name: "Fabulously Optimized",
    author: "robotkoer",
    iconUrl: null,
  });
  listModpackVersions.mockResolvedValue([
    { id: "v14", name: "14.0.0", publishedAt: null },
    { id: "v15", name: "15.0.0-beta.3", publishedAt: "2020-01-01T00:00:00Z" },
  ]);
}

describe("ModpackContentCard", () => {
  beforeEach(() => {
    getModpack.mockReset();
    listModpackVersions.mockReset();
  });

  it("shows what the modpack ships and which pack and version provide it", async () => {
    mockStore();

    renderCard(STORE);

    const card = screen.getByRole("region", { name: "Modpack content" });
    expect(
      screen.getByRole("heading", { name: "Modpack content" }),
    ).toBeInTheDocument();
    expect(screen.getByText("2 mods")).toBeInTheDocument();
    expect(screen.getByText("1 resource pack")).toBeInTheDocument();
    await waitFor(() =>
      expect(card).toHaveTextContent(
        "Provided byFabulously Optimized· 15.0.0-beta.3",
      ),
    );
    // Author and release age are shown in the content dialog instead.
    expect(card).not.toHaveTextContent(/robotkoer|Updated/);
    expect(getModpack).toHaveBeenCalledWith("modrinth", "fo");
  });

  it("opens the pack's page from its name", async () => {
    mockStore();
    const { onOpenModpack } = renderCard(STORE, ENTRIES, true);

    fireEvent.click(
      await screen.findByRole("button", { name: "Fabulously Optimized" }),
    );

    expect(onOpenModpack).toHaveBeenCalledOnce();
  });

  it("still shows what the pack ships when the store cannot be reached", async () => {
    getModpack.mockRejectedValue(new Error("offline"));
    listModpackVersions.mockRejectedValue(new Error("offline"));

    renderCard(STORE);

    await waitFor(() => expect(getModpack).toHaveBeenCalled());
    expect(screen.getByText("2 mods")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "View content" })).toBeEnabled();
  });

  it("says a pack imported from disk came from a file, without asking a store", () => {
    renderCard({ type: "modpackFile", source: "modrinth" }, ENTRIES, true);

    expect(
      screen.getByText("Imported from a Modrinth file"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Provided/ }),
    ).not.toBeInTheDocument();
    expect(getModpack).not.toHaveBeenCalled();
  });

  it("asks to view the modpack content", () => {
    const { onView } = renderCard({
      type: "modpackFile",
      source: "modrinth",
    });

    fireEvent.click(screen.getByRole("button", { name: "View content" }));

    expect(onView).toHaveBeenCalledOnce();
  });

  it("cannot view content when the modpack ships none", () => {
    renderCard({ type: "modpackFile", source: "modrinth" }, []);

    expect(screen.getByRole("button", { name: "View content" })).toBeDisabled();
  });
});
