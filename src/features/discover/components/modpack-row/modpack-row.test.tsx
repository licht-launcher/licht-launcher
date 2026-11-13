import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ModpackRow } from "@/features/discover/components/modpack-row";
import { modpackListItem } from "@/features/discover/fixtures/modpacks";
import type { ModpackListItem } from "@/lib/generated/ModpackListItem";

const NOW = Date.parse("2026-11-04T12:00:00Z");

function renderRow(modpack: ModpackListItem) {
  const router = createRouter({
    routeTree: createRootRoute({
      component: () => <ModpackRow modpack={modpack} now={NOW} />,
    }),
    history: createMemoryHistory({ initialEntries: ["/discover"] }),
  });
  return render(<RouterProvider router={router} />);
}

describe("ModpackRow", () => {
  it("links the whole result to the pack's store page", async () => {
    renderRow(modpackListItem());

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    expect(link).toHaveAttribute("href", "/discover/modrinth/1KVo5zza");
    expect(within(link).getByText("by robotkoer")).toBeInTheDocument();
    expect(within(link).getByText("19.2M Downloads")).toBeInTheDocument();
    expect(within(link).queryByText("Modrinth")).not.toBeInTheDocument();
  });

  it("shows a placeholder icon and no author when the source has none", async () => {
    renderRow(
      modpackListItem({ source: "curseforge", iconUrl: null, author: "" }),
    );

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    expect(within(link).queryByRole("img")).not.toBeInTheDocument();
    expect(within(link).queryByText(/^by /)).not.toBeInTheDocument();
  });

  it("shows up to three categories and when it was updated", async () => {
    renderRow(modpackListItem({ loaders: ["fabric", "neoforge"] }));

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    // Loaders are left to the pack's page.
    expect(within(link).queryByText("Fabric")).not.toBeInTheDocument();
    for (const category of ["Lightweight", "Multiplayer", "Optimization"]) {
      expect(within(link).getByText(category)).toHaveAttribute("data-category");
    }
    expect(within(link).queryByText("Quests")).not.toBeInTheDocument();
    const updated = within(link).getByText("Updated 3 days ago");
    expect(updated).toHaveAttribute("datetime", "2026-11-01T10:43:19.109719Z");
    expect(updated).toHaveAttribute("title", "Nov 1, 2026");
  });

  it("leaves out categories and update time the source did not send", async () => {
    renderRow(
      modpackListItem({ loaders: [], categories: [], updatedAt: null }),
    );

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    expect(within(link).queryByText(/Updated/)).not.toBeInTheDocument();
    expect(within(link).queryByText("Lightweight")).not.toBeInTheDocument();
  });
});
