import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  modpackDetails,
  modpackSearchPage,
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
    // Going back to the store list asks for the sources, then searches.
    if (command === "modpack_sources") {
      return Promise.resolve(["modrinth", "curseforge"]);
    }
    if (command === "search_modpacks") {
      return Promise.resolve(modpackSearchPage());
    }
    return undefined;
  });
}

describe("Modpack page", () => {
  setupInstancesPageTests();

  it("shows the pack's header, description and sidebar", async () => {
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
    expect(screen.getByText("5.1K followers")).toBeInTheDocument();
    expect(screen.getByText(/^Updated /, { selector: "time" })).toHaveAttribute(
      "title",
      "Nov 1, 2026",
    );
    const categories = screen.getByRole("list", { name: "Categories" });
    expect(within(categories).getAllByRole("listitem")).toHaveLength(2);
    expect(
      within(screen.getByRole("tabpanel")).getByRole("heading", {
        name: "Features",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("complementary", { name: "About this modpack" }),
    ).toHaveTextContent("Compatibility");
    expect(invoke).toHaveBeenCalledWith("get_modpack", {
      source: "modrinth",
      projectId: "1KVo5zza",
    });
  });

  it("counts versions on their tab and shows the gallery on its own", async () => {
    mockModpack(() => Promise.resolve(modpackDetails()));
    await renderAt("/discover/modrinth/1KVo5zza");

    expect(
      await screen.findByRole("tab", { name: "Versions 2" }),
    ).toBeInTheDocument();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Gallery" }));

    const gallery = await screen.findByRole("list", { name: "Gallery" });
    expect(
      within(gallery).getByRole("img", { name: "Title screen" }),
    ).toHaveAttribute(
      "src",
      "https://cdn.modrinth.com/data/1KVo5zza/images/title.webp",
    );
  });

  it("has no gallery tab when the pack has no images", async () => {
    mockModpack(() => Promise.resolve(modpackDetails({ gallery: [] })));
    await renderAt("/discover/modrinth/1KVo5zza");

    await screen.findByRole("tab", { name: "Description" });
    expect(
      screen.queryByRole("tab", { name: "Gallery" }),
    ).not.toBeInTheDocument();
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

  it("starts a fresh install form when moving to another pack", async () => {
    mockContextMenuCommands((command, args) => {
      if (command === "get_modpack") {
        const projectId =
          args !== null && typeof args === "object" && "projectId" in args
            ? String(args.projectId)
            : "";
        return Promise.resolve(
          modpackDetails({
            id: projectId,
            name: projectId === "other" ? "Other Pack" : "Fabulously Optimized",
          }),
        );
      }
      if (command === "list_modpack_versions") {
        return Promise.resolve(modpackVersions());
      }
      return undefined;
    });
    const { router } = await renderAt("/discover/modrinth/other");
    await screen.findByRole("heading", { level: 1, name: "Other Pack" });
    await router.navigate({
      to: "/discover/$source/$projectId",
      params: { source: "modrinth", projectId: "1KVo5zza" },
    });
    fireEvent.click(await screen.findByRole("button", { name: "Install" }));
    fireEvent.change(
      within(
        await screen.findByRole("dialog", {
          name: "Install Fabulously Optimized",
        }),
      ).getByRole("textbox", { name: "Instance Name" }),
      { target: { value: "My pack" } },
    );

    // The other pack is cached, so its page shows without a loading state.
    await router.navigate({
      to: "/discover/$source/$projectId",
      params: { source: "modrinth", projectId: "other" },
    });

    await screen.findByRole("heading", { level: 1, name: "Other Pack" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Install" }));
    expect(
      within(
        await screen.findByRole("dialog", { name: "Install Other Pack" }),
      ).getByRole("textbox", { name: "Instance Name" }),
    ).toHaveValue("");
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
