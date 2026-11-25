import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EditInstanceBaseDialog } from "@/features/instance-settings/components/edit-instance-base-dialog";
import type { LoaderBuild } from "@/lib/generated/LoaderBuild";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const builds: LoaderBuild[] = [
  { version: "0.16.14", stable: true },
  { version: "0.16.10", stable: true },
];

function renderDialog(
  props: Partial<ComponentProps<typeof EditInstanceBaseDialog>> = {},
) {
  const onSave = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <EditInstanceBaseDialog
        open
        current={{
          platform: "fabric",
          gameVersion: "1.21.1",
          loaderVersion: "0.16.10",
        }}
        busy={false}
        onOpenChange={() => {}}
        onSave={onSave}
        {...props}
      />
    </QueryClientProvider>,
  );
  return { onSave };
}

describe("EditInstanceBaseDialog", () => {
  beforeEach(() => {
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        return this.dataset.slot === "combobox-viewport" ? 256 : 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(320);
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "list_versions") {
        return Promise.resolve([
          { id: "1.21.4", versionType: "release", installed: false },
          { id: "1.21.1", versionType: "release", installed: true },
        ]);
      }
      if (command === "list_loader_builds") return Promise.resolve(builds);
      return Promise.reject(new Error(`unexpected ${command}`));
    });
  });

  afterEach(() => vi.restoreAllMocks());

  it("starts from the current base and saves nothing until it changes", async () => {
    renderDialog();

    const version = await screen.findByRole("combobox", {
      name: "Game version",
    });
    await waitFor(() => expect(version).toHaveValue("1.21.1"));
    expect(
      screen.getByRole("button", { name: /^Fabric/, pressed: true }),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_loader_builds", {
        platform: "fabric",
        gameVersion: "1.21.1",
      });
    });
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

  it("saves another game version with the loader channel picked", async () => {
    const { onSave } = renderDialog();

    const field = await screen.findByRole("combobox", { name: "Game version" });
    await waitFor(() => expect(field).toHaveValue("1.21.1"));
    fireEvent.focus(field);
    fireEvent.change(field, { target: { value: "1.21.4" } });
    fireEvent.click(await screen.findByRole("option", { name: "1.21.4" }));
    fireEvent.click(screen.getByRole("button", { name: /^Stable/ }));
    const save = screen.getByRole("button", { name: "Save" });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);

    expect(onSave).toHaveBeenCalledWith({
      platform: "fabric",
      gameVersion: "1.21.4",
      loaderChannel: "stable",
      loaderVersion: null,
    });
  });

  it("warns that mods stop loading when the platform changes", async () => {
    const { onSave } = renderDialog();

    fireEvent.click(await screen.findByRole("button", { name: /^Vanilla/ }));

    expect(screen.getByRole("note")).toHaveTextContent(
      "Mods made for another platform will not load after the change.",
    );
    const save = screen.getByRole("button", { name: "Save" });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);
    expect(onSave).toHaveBeenCalledWith({
      platform: "vanilla",
      gameVersion: "1.21.1",
      loaderChannel: "stable",
      loaderVersion: null,
    });
  });
});
