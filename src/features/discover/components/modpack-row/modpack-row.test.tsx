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
    expect(within(link).queryByText("Modrinth")).not.toBeInTheDocument();
    expect(link).toHaveTextContent("19.2M downloads");
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
    expect(within(link).queryByText("CurseForge")).not.toBeInTheDocument();
  });

  it("shows loaders, up to three categories and when it was updated", async () => {
    renderRow(modpackListItem({ loaders: ["fabric", "neoforge"] }));

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    const fabric = within(link).getByText("Fabric");
    const neoforge = within(link).getByText("NeoForge");
    expect(fabric).toHaveAttribute("data-slot", "badge");
    expect(fabric).toHaveAttribute("data-loader", "fabric");
    expect(neoforge).toHaveAttribute("data-loader", "neoforge");
    // Each loader gets its own color.
    expect(fabric.className).not.toBe(neoforge.className);
    const logos = [...link.querySelectorAll("img")].map(
      (image) => image.getAttribute("src") ?? "",
    );
    expect(logos.some((src) => src.includes("fabric"))).toBe(true);
    expect(logos.some((src) => src.includes("neoforge"))).toBe(true);
    for (const category of ["Lightweight", "Multiplayer", "Optimization"]) {
      expect(within(link).getByText(category)).toBeInTheDocument();
    }
    expect(within(link).queryByText("Quests")).not.toBeInTheDocument();
    const updated = within(link).getByText("Updated 3 days ago");
    expect(updated).toHaveAttribute("datetime", "2026-11-01T10:43:19.109719Z");
    expect(updated).toHaveAttribute("title", "Nov 1, 2026");
  });

  it("leaves out tags and update time the source did not send", async () => {
    renderRow(
      modpackListItem({ loaders: [], categories: [], updatedAt: null }),
    );

    const link = await screen.findByRole("link", {
      name: /Fabulously Optimized/,
    });
    expect(within(link).queryByText(/Updated/)).not.toBeInTheDocument();
    expect(within(link).queryByText("Fabric")).not.toBeInTheDocument();
  });
});
