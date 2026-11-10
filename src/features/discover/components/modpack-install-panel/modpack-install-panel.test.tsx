import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  modpackDetails,
  modpackVersions,
} from "@/features/discover/fixtures/modpacks";
import {
  classicInstance,
  invoke,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";

const createdInstance = {
  ...classicInstance,
  folder: "fabulously-optimized",
  name: "Fabulously Optimized",
};

function mockInstall(
  versions: () => Promise<unknown> = () => Promise.resolve(modpackVersions()),
) {
  mockContextMenuCommands((command) => {
    if (command === "get_modpack") return Promise.resolve(modpackDetails());
    if (command === "list_modpack_versions") return versions();
    if (command === "create_instance_from_modpack") {
      return Promise.resolve(createdInstance);
    }
    return undefined;
  });
}

async function findInstallPanel() {
  return screen.findByRole("region", { name: "Install" });
}

describe("ModpackInstallPanel", () => {
  setupInstancesPageTests();

  it("installs the newest version under the pack's name and opens it", async () => {
    mockInstall();
    const { router } = await renderAt("/discover/modrinth/1KVo5zza");
    const panel = await findInstallPanel();

    expect(
      await within(panel).findByRole("combobox", { name: "Version" }),
    ).toHaveTextContent("7.0.0 (1.21.10 · fabric)");
    expect(
      within(panel).getByRole("textbox", { name: "Instance Name" }),
    ).toHaveAttribute("placeholder", "Fabulously Optimized");
    fireEvent.click(within(panel).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack", {
        name: "Fabulously Optimized",
        source: "modrinth",
        projectId: "1KVo5zza",
        versionId: "v7",
        iconUrl: "https://cdn.modrinth.com/data/1KVo5zza/icon.webp",
      }),
    );
    await waitFor(() =>
      expect(router.state.location.search).toEqual({
        prepare: "fabulously-optimized",
      }),
    );
    expect(router.state.location.pathname).toBe("/");
  });

  it("uses a typed name as given", async () => {
    mockInstall();
    await renderAt("/discover/modrinth/1KVo5zza");
    const panel = await findInstallPanel();

    await within(panel).findByRole("combobox", { name: "Version" });
    fireEvent.change(
      within(panel).getByRole("textbox", { name: "Instance Name" }),
      { target: { value: "  My pack  " } },
    );
    fireEvent.click(within(panel).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ name: "My pack" }),
      ),
    );
  });

  it("cannot install a pack without versions", async () => {
    mockInstall(() => Promise.resolve([]));
    await renderAt("/discover/modrinth/1KVo5zza");
    const panel = await findInstallPanel();

    expect(
      await within(panel).findByText("This pack has no versions to install."),
    ).toBeInTheDocument();
    expect(
      within(panel).getByRole("button", { name: "Install" }),
    ).toBeDisabled();
  });

  it("shows why versions could not be listed", async () => {
    mockInstall(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover/modrinth/1KVo5zza");
    const panel = await findInstallPanel();

    expect(await within(panel).findByRole("alert")).toHaveTextContent(
      "The network request failed. Check your connection and try again.",
    );
    expect(
      within(panel).getByRole("button", { name: "Install" }),
    ).toBeDisabled();
  });
});
