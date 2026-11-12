import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { mockListLayout } from "@/features/discover/fixtures/list-layout";
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

/** Opens the dialog from the header's Install button. */
async function openInstallDialog() {
  fireEvent.click(await screen.findByRole("button", { name: "Install" }));
  return screen.findByRole("dialog", { name: "Install Fabulously Optimized" });
}

describe("ModpackInstallDialog", () => {
  setupInstancesPageTests();

  it("installs the newest version under the pack's name and opens it", async () => {
    mockInstall();
    const { router } = await renderAt("/discover/modrinth/1KVo5zza");
    const dialog = await openInstallDialog();

    expect(
      await within(dialog).findByRole("combobox", { name: "Version" }),
    ).toHaveTextContent("7.0.0 (1.21.10 · Fabric)");
    expect(
      within(dialog).getByRole("textbox", { name: "Instance Name" }),
    ).toHaveAttribute("placeholder", "Fabulously Optimized");
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

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

  it("selects the newest release rather than a newer beta", async () => {
    const [release, older] = modpackVersions();
    mockInstall(() =>
      Promise.resolve([
        {
          ...older,
          id: "v8-beta",
          name: "8.0.0-beta",
          versionType: "beta",
        },
        release,
        older,
      ]),
    );
    await renderAt("/discover/modrinth/1KVo5zza");
    const dialog = await openInstallDialog();

    expect(
      await within(dialog).findByRole("combobox", { name: "Version" }),
    ).toHaveTextContent("7.0.0 (1.21.10 · Fabric)");
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ versionId: "v7" }),
      ),
    );
  });

  it("uses a typed name as given", async () => {
    mockInstall();
    await renderAt("/discover/modrinth/1KVo5zza");
    const dialog = await openInstallDialog();

    await within(dialog).findByRole("combobox", { name: "Version" });
    fireEvent.change(
      within(dialog).getByRole("textbox", { name: "Instance Name" }),
      { target: { value: "  My pack  " } },
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ name: "My pack" }),
      ),
    );
  });

  it("opens on the version picked in the Versions tab", async () => {
    mockListLayout();
    mockInstall();
    await renderAt("/discover/modrinth/1KVo5zza");

    fireEvent.mouseDown(await screen.findByRole("tab", { name: /Versions/ }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Install 6.4.0" }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Install Fabulously Optimized",
    });

    expect(
      within(dialog).getByRole("combobox", { name: "Version" }),
    ).toHaveTextContent("6.4.0 (1.21.8 · Fabric)");
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_instance_from_modpack",
        expect.objectContaining({ versionId: "v6" }),
      ),
    );
  });

  it("cannot install a pack without versions", async () => {
    mockInstall(() => Promise.resolve([]));
    await renderAt("/discover/modrinth/1KVo5zza");
    const dialog = await openInstallDialog();

    expect(
      await within(dialog).findByText("This pack has no versions to install."),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByRole("button", { name: "Install" }),
    ).toBeDisabled();
  });

  it("shows why versions could not be listed", async () => {
    mockInstall(() => Promise.reject("HTTP request failed"));
    await renderAt("/discover/modrinth/1KVo5zza");
    const dialog = await openInstallDialog();

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(
      "The network request failed. Check your connection and try again.",
    );
    expect(
      within(dialog).getByRole("button", { name: "Install" }),
    ).toBeDisabled();
  });
});
