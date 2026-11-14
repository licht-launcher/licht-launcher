import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { mockListLayout } from "@/features/discover/fixtures/list-layout";
import { modpackVersions } from "@/features/discover/fixtures/modpacks";
import type { ModpackVersionItem } from "@/lib/generated/ModpackVersionItem";

import { ModpackVersions } from "./modpack-versions";

const { listModpackVersions } = vi.hoisted(() => ({
  listModpackVersions: vi.fn<() => Promise<ModpackVersionItem[]>>(),
}));

vi.mock("@/api/commands", () => ({ listModpackVersions }));

function renderVersions(onInstall = vi.fn(), installing = false) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ModpackVersions
        source="modrinth"
        projectId="1KVo5zza"
        installing={installing}
        onInstall={onInstall}
      />
    </QueryClientProvider>,
  );
  return onInstall;
}

describe("ModpackVersions", () => {
  it("lists every version with its type, game versions, loaders and downloads", async () => {
    mockListLayout();
    listModpackVersions.mockResolvedValue(modpackVersions());
    renderVersions();

    const table = await screen.findByRole("table", { name: "Versions" });
    const [, newest, older] = within(table).getAllByRole("row");
    expect(newest).toHaveTextContent("7.0.0");
    expect(within(newest as HTMLElement).getByText("Release")).toHaveAttribute(
      "data-version-type",
      "release",
    );
    expect(newest).toHaveTextContent("1.21.10");
    expect(within(newest as HTMLElement).getByText("Fabric")).toHaveAttribute(
      "data-loader",
      "fabric",
    );
    expect(newest).toHaveTextContent("160.4K");
    expect((newest as HTMLElement).querySelector("time")).toHaveAttribute(
      "title",
      "Nov 1, 2026",
    );
    expect(within(older as HTMLElement).getByText("Beta")).toBeInTheDocument();
  });

  it("installs the version of the row that was clicked", async () => {
    mockListLayout();
    listModpackVersions.mockResolvedValue(modpackVersions());
    const onInstall = renderVersions();

    fireEvent.click(
      await screen.findByRole("button", { name: "Install 6.4.0" }),
    );

    expect(onInstall).toHaveBeenCalledWith("v6");
  });

  it("does not install a row while an install is already running", async () => {
    mockListLayout();
    listModpackVersions.mockResolvedValue(modpackVersions());
    const onInstall = renderVersions(vi.fn(), true);

    const button = await screen.findByRole("button", { name: "Install 6.4.0" });
    expect(button).toBeDisabled();
    fireEvent.click(button);

    expect(onInstall).not.toHaveBeenCalled();
  });

  it("says when the pack has no versions", async () => {
    listModpackVersions.mockResolvedValue([]);
    renderVersions();

    expect(
      await screen.findByRole("status", { name: "No versions yet" }),
    ).toBeInTheDocument();
  });

  it("explains when versions cannot be loaded", async () => {
    listModpackVersions.mockRejectedValue("HTTP request failed");
    renderVersions();

    expect(
      await screen.findByRole("status", { name: "Could not load versions" }),
    ).toHaveTextContent(
      "The network request failed. Check your connection and try again.",
    );
  });
});
