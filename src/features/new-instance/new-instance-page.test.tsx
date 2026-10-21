import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Toaster } from "@/components/ui/sonner";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import { startupProgress } from "@/features/startup/fixtures/progress";
import { routeTree } from "@/routeTree.gen";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    close: vi.fn(),
    setDecorations: vi.fn(),
  }),
}));

async function renderWizard(initialEntry = "/instances/new") {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [initialEntry] }),
  });
  await router.load();
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
}

/** Wait until a choice-card selection is reflected in the URL search. */
async function selectChoice(name: string | RegExp) {
  const button = await screen.findByRole("button", { name });
  fireEvent.click(button);
  await waitFor(() => {
    expect(screen.getByRole("button", { name })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
}

async function clickContinue() {
  const button = screen.getByRole("button", { name: "Continue" });
  await waitFor(() => {
    expect(button).not.toBeDisabled();
  });
  fireEvent.click(button);
}

async function clickContinueTo(nextChoice: string | RegExp) {
  await clickContinue();
  expect(await screen.findByRole("button", { name: nextChoice })).toBeTruthy();
}

async function clickContinueUntil(find: () => Promise<unknown>) {
  await clickContinue();
  await find();
}

describe("NewInstancePage", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "get_startup_progress")
        return Promise.resolve(startupProgress());
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
          { id: "24w14a", versionType: "snapshot", installed: false },
        ]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve(null);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      if (command === "list_loader_versions") {
        return Promise.resolve(["0.16.0", "0.15.11"]);
      }
      if (command === "search_modpacks") {
        return Promise.resolve([
          {
            id: "ABC",
            source: "modrinth",
            name: "Cobblemon",
            description: "Pokemon",
            downloads: 10,
            iconUrl: null,
            author: "team",
          },
        ]);
      }
      if (command === "list_modpack_versions") {
        return Promise.resolve([
          {
            id: "v1",
            name: "1.0.0",
            gameVersions: ["1.20.1"],
            loaders: ["fabric"],
          },
        ]);
      }
      if (command === "create_instance_with_platform") {
        return Promise.resolve({
          folder: "Survival",
          name: "Survival",
          versionId: "1.20.1",
          iconPath: null,
          lastPlayed: null,
        });
      }
      if (command === "create_instance_from_modpack") {
        return Promise.resolve({
          folder: "Cobblemon",
          name: "Cobblemon",
          versionId: "1.20.1",
          iconPath: null,
          lastPlayed: null,
        });
      }
      return Promise.resolve(undefined);
    });
  });

  it("restores progress from the URL search params", async () => {
    await renderWizard(
      "/instances/new?step=details&source=scratch&platform=vanilla&name=Survival&gameVersion=1.20.1",
    );
    expect(await screen.findByLabelText("Instance Name")).toHaveValue(
      "Survival",
    );
    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("creates a vanilla instance through the wizard", async () => {
    await renderWizard();

    expect(
      screen.getByRole("navigation", { name: "Instances" }),
    ).toBeInTheDocument();
    await selectChoice(/From scratch/);
    await clickContinueTo(/^Vanilla/);
    await selectChoice(/^Vanilla/);
    await clickContinueUntil(() => screen.findByLabelText("Instance Name"));

    const nameInput = await screen.findByLabelText("Instance Name");
    expect(nameInput).toHaveAttribute("placeholder", "Survival");
    fireEvent.change(nameInput, {
      target: { value: "Survival" },
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Instance Name")).toHaveValue("Survival");
    });
    expect(
      await screen.findByRole("button", { name: "1.20 versions" }),
    ).toBeInTheDocument();
    const releaseOption = await screen.findByRole("button", {
      name: "1.20.1",
    });
    expect(
      screen.queryByRole("button", { name: /24w14a/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(releaseOption);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "1.20.1" })).toHaveAttribute(
        "aria-pressed",
        "true",
      );
    });

    await clickContinueUntil(() =>
      screen.findByRole("button", { name: "Create instance" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create instance" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Survival",
        platform: "vanilla",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      });
    });
  });

  it("includes snapshots when the snapshots button is on", async () => {
    await renderWizard();

    await selectChoice(/From scratch/);
    await clickContinueTo(/^Vanilla/);
    await selectChoice(/^Vanilla/);
    await clickContinueUntil(() => screen.findByLabelText("Instance Name"));

    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /24w14a/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "1.20.1" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "1.20.1" })).toHaveAttribute(
        "aria-pressed",
        "true",
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "Include snapshots" }));
    expect(
      await screen.findByRole("button", { name: "Snapshots versions" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: "24w14a" }),
    ).toBeInTheDocument();
  });

  it("filters the version list from the search field", async () => {
    await renderWizard();

    await selectChoice(/From scratch/);
    await clickContinueTo(/^Vanilla/);
    await selectChoice(/^Vanilla/);
    await clickContinueUntil(() => screen.findByLabelText("Instance Name"));

    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Search versions"), {
      target: { value: "nope" },
    });
    expect(await screen.findByText("No versions match.")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Search versions"), {
      target: { value: "1.20" },
    });
    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toBeInTheDocument();
  });

  it("asks for a fabric loader channel on the details step", async () => {
    await renderWizard();

    await selectChoice(/From scratch/);
    await clickContinueTo(/^Fabric/);
    await selectChoice(/^Fabric/);
    await clickContinueUntil(() =>
      screen.findByRole("combobox", { name: "Game version" }),
    );

    await screen.findByRole("combobox", { name: "Game version" });
    expect(
      screen.getByRole("combobox", { name: "Loader channel" }),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Instance Name"), {
      target: { value: "Modded" },
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Instance Name")).toHaveValue("Modded");
    });
    await clickContinueUntil(() =>
      screen.findByRole("button", { name: "Create instance" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create instance" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_instance_with_platform", {
        name: "Modded",
        platform: "fabric",
        gameVersion: "1.20.1",
        loaderChannel: "stable",
        loaderVersion: null,
      });
    });
  });

  it("imports a modrinth modpack", async () => {
    await renderWizard();

    await selectChoice(/From a modpack/);
    await clickContinueUntil(() => screen.findByLabelText("Search modpacks"));

    fireEvent.change(await screen.findByLabelText("Search modpacks"), {
      target: { value: "cobble" },
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Search modpacks")).toHaveValue("cobble");
    });
    fireEvent.click(await screen.findByRole("button", { name: /Cobblemon/ }));
    await clickContinueUntil(() => screen.findByLabelText(/^Name$/));

    expect(await screen.findByLabelText(/^Name$/)).toHaveValue("Cobblemon");
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Continue" }),
      ).not.toBeDisabled();
    });
    await clickContinueUntil(() =>
      screen.findByRole("button", { name: "Create instance" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create instance" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_instance_from_modpack", {
        name: "Cobblemon",
        source: "modrinth",
        projectId: "ABC",
        versionId: "v1",
        iconUrl: null,
      });
    });
  });

  it("shows retry when the version list fails", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "get_startup_progress")
        return Promise.resolve(startupProgress());
      if (command === "list_versions") {
        return Promise.reject(new Error("offline"));
      }
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve(null);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      return Promise.resolve(undefined);
    });

    await renderWizard();
    await selectChoice(/From scratch/);
    await clickContinueTo(/^Vanilla/);
    await selectChoice(/^Vanilla/);
    await clickContinueUntil(() =>
      screen.findByText(/Could not load the version list/),
    );

    expect(
      await screen.findByText(/Could not load the version list/),
    ).toBeInTheDocument();

    invoke.mockImplementation((command: string) => {
      if (command === "get_startup_progress")
        return Promise.resolve(startupProgress());
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.20.1", versionType: "release", installed: false },
        ]);
      }
      if (command === "list_instances") {
        return Promise.resolve([]);
      }
      if (command === "get_settings") {
        return Promise.resolve(snapshot);
      }
      if (command === "get_last_played_folder") {
        return Promise.resolve(null);
      }
      if (command === "list_running_games") {
        return Promise.resolve([]);
      }
      if (command === "custom_title_bar_active") {
        return Promise.resolve(true);
      }
      return Promise.resolve(undefined);
    });

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(
      await screen.findByRole("button", { name: "1.20.1" }),
    ).toBeInTheDocument();
  });
});
