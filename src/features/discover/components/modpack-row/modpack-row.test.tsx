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

function renderRow(modpack: ModpackListItem) {
  const router = createRouter({
    routeTree: createRootRoute({
      component: () => <ModpackRow modpack={modpack} />,
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
    expect(within(link).getByText("Modrinth")).toBeInTheDocument();
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
    expect(within(link).getByText("CurseForge")).toBeInTheDocument();
  });
});
