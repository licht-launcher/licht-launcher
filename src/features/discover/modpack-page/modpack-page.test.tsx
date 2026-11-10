import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  modpackDetails,
  modpackVersions,
} from "@/features/discover/fixtures/modpacks";
import {
  invoke,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";

function mockModpack(details: () => Promise<unknown>) {
  mockContextMenuCommands((command) => {
    if (command === "get_modpack") return details();
    if (command === "list_modpack_versions") {
      return Promise.resolve(modpackVersions());
    }
    if (command === "open_external_url") return Promise.resolve();
    return undefined;
  });
}

describe("Modpack page", () => {
  setupInstancesPageTests();

  it("shows the pack's details, gallery and description", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Fabulously Optimized",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("by robotkoer")).toBeInTheDocument();
    expect(screen.getByText("19.2M downloads")).toBeInTheDocument();
    expect(screen.getByText("Updated Nov 1, 2026")).toBeInTheDocument();
    const categories = screen.getByRole("list", { name: "Categories" });
    expect(within(categories).getAllByRole("listitem")).toHaveLength(2);
    const gallery = screen.getByRole("region", { name: "Gallery" });
    expect(gallery.querySelector("img")).toHaveAttribute(
      "src",
      "https://cdn.modrinth.com/data/1KVo5zza/images/title.webp",
    );
    const description = screen.getByRole("region", { name: "Description" });
    expect(
      within(description).getByRole("heading", { name: "Features" }),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("get_modpack", {
      source: "modrinth",
      projectId: "1KVo5zza",
    });
  });

  it("opens the project page in the system browser", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.click(
      await screen.findByRole("button", { name: "Open on Modrinth" }),
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("open_external_url", {
        url: "https://modrinth.com/modpack/fabulously-optimized",
      }),
    );
  });

  it("goes back to the store list", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    const { router } = await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.click(await screen.findByRole("link", { name: "Discover" }));

    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/discover"),
    );
  });

  it("explains when the pack cannot be loaded", async () => {
    mockModpack(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("status", {
        name: "Could not load this modpack",
      }),
    ).toBeInTheDocument();
  });

  it("rejects links with an unknown source without calling the backend", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/ftb/123");

    expect(
      await screen.findByRole("status", { name: "Modpack not found" }),
    ).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("get_modpack", expect.anything());
  });
});
