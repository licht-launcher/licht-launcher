import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";
import type { InstanceContentKind } from "@/lib/generated/InstanceContentKind";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

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

const LINKED: LinkedModpack = {
  name: "Fabulously Optimized",
  author: "robotkoer",
  versionName: "15.0.0-beta.3",
  publishedAt: "2020-01-01T00:00:00Z",
};

function renderCard(
  origin: ModpackOrigin,
  entries: InstanceContentEntry[] = ENTRIES,
  linked: LinkedModpack | null = null,
) {
  const onView = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ModpackContentCard
        origin={origin}
        linked={linked}
        iconPath={null}
        entries={entries}
        onView={onView}
      />
    </QueryClientProvider>,
  );
  return { onView };
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

  it("names the pack with its version and release age", async () => {
    mockStore();

    renderCard(STORE);

    expect(
      await screen.findByRole("heading", { name: "Fabulously Optimized" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("15.0.0-beta.3")).toBeInTheDocument();
    expect(screen.getByText("Version").nextElementSibling).toHaveTextContent(
      "15.0.0-beta.3",
    );
    expect(screen.getByText("Updated").nextElementSibling).toHaveTextContent(
      /ago$/,
    );
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual(
      ["Version", "Updated"],
    );
    // The pack name is not a link, and the author lives in the dialog.
    expect(
      screen.queryByRole("button", { name: "Fabulously Optimized" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: "Modpack content" }),
    ).not.toHaveTextContent(/robotkoer/);
    expect(getModpack).toHaveBeenCalledWith("modrinth", "fo");
  });

  it("shows the details saved at install right away, without asking the store", () => {
    renderCard(STORE, ENTRIES, LINKED);

    expect(
      screen.getByRole("heading", { name: "Fabulously Optimized" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Version").nextElementSibling).toHaveTextContent(
      "15.0.0-beta.3",
    );
    expect(screen.getByText("Updated").nextElementSibling).toHaveTextContent(
      /ago$/,
    );
    expect(getModpack).not.toHaveBeenCalled();
    expect(listModpackVersions).not.toHaveBeenCalled();
  });

  it("still opens its content when the store cannot be reached", async () => {
    getModpack.mockRejectedValue(new Error("offline"));
    listModpackVersions.mockRejectedValue(new Error("offline"));

    renderCard(STORE);

    await waitFor(() => expect(getModpack).toHaveBeenCalled());
    expect(screen.getByText("Version").nextElementSibling).toHaveTextContent(
      "Unknown",
    );
    expect(screen.getByRole("button", { name: "View content" })).toBeEnabled();
  });

  it("says a pack imported from disk came from a file, without asking a store", () => {
    renderCard({ type: "modpackFile", source: "modrinth" });

    expect(
      screen.getByRole("heading", { name: "Imported from a Modrinth file" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Version")).not.toBeInTheDocument();
    expect(screen.queryByText("Updated")).not.toBeInTheDocument();
    expect(getModpack).not.toHaveBeenCalled();
  });

  it("names an imported pack and its version from what its archive declared", () => {
    renderCard({ type: "modpackFile", source: "modrinth" }, ENTRIES, {
      name: "Fresh & Smooth",
      author: null,
      versionName: "2.1.0",
      publishedAt: null,
    });

    expect(
      screen.getByRole("heading", { name: "Fresh & Smooth" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Version").nextElementSibling).toHaveTextContent(
      "2.1.0",
    );
    expect(screen.queryByText("Updated")).not.toBeInTheDocument();
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
