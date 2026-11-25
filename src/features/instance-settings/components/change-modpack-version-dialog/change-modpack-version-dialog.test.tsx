import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ChangeModpackVersionDialog } from "@/features/instance-settings/components/change-modpack-version-dialog";
import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function version(id: string): ModpackVersionItem {
  return {
    id,
    name: `Pack ${id}`,
    gameVersions: ["1.21.1"],
    loaders: ["fabric"],
    versionType: "release",
    publishedAt: null,
    downloads: 0,
  };
}

function renderDialog() {
  const onSave = vi.fn();
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ChangeModpackVersionDialog
        open
        origin={{
          type: "modpack",
          source: "modrinth",
          projectId: "fo",
          versionId: "v15",
          iconUrl: null,
        }}
        busy={false}
        onOpenChange={() => {}}
        onSave={onSave}
      />
    </QueryClientProvider>,
  );
  return { onSave };
}

describe("ChangeModpackVersionDialog", () => {
  beforeEach(() => invoke.mockReset());

  it("marks the installed version and installs another one", async () => {
    invoke.mockResolvedValue([version("v16"), version("v15")]);
    const { onSave } = renderDialog();

    const install = await screen.findByRole("button", {
      name: "Install version",
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Version")).toHaveTextContent(
        "Pack v15 (installed)",
      );
    });
    expect(install).toBeDisabled();
    fireEvent.click(screen.getByLabelText("Version"));
    fireEvent.click(await screen.findByRole("option", { name: "Pack v16" }));
    fireEvent.click(install);

    expect(onSave).toHaveBeenCalledWith("v16");
    expect(invoke).toHaveBeenCalledWith("list_modpack_versions", {
      source: "modrinth",
      projectId: "fo",
    });
  });

  it("offers a retry when the versions cannot be listed", async () => {
    invoke
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValue([version("v16"), version("v15")]);
    renderDialog();

    fireEvent.click(await screen.findByRole("button", { name: "Retry" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Version")).toHaveTextContent("Pack v15");
    });
  });
});
