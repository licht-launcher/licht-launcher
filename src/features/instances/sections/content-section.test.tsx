import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ContentSection } from "@/features/instances/sections/content-section";

const listInstanceContent = vi.hoisted(() => vi.fn());

vi.mock("@/lib/commands", () => ({
  listInstanceContent,
  removeInstanceContent: vi.fn(),
  setInstanceContentEnabled: vi.fn(),
}));

function renderContent(
  folder: string | null,
  filter: "all" | "mods" | "resourcePacks" | "shaderPacks" = "all",
) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ContentSection folder={folder} filter={filter} search="" />
    </QueryClientProvider>,
  );
}

describe("ContentSection", () => {
  beforeEach(() => {
    listInstanceContent.mockReset();
  });

  it("asks to select an instance when none is selected", () => {
    renderContent(null);
    expect(
      screen.getByText("Select an instance to manage its content."),
    ).toBeInTheDocument();
  });

  it("formats titles and filters the combined content list", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "sodium-fabric-1.0.jar",
        kind: "mods",
        enabled: true,
        size: 1024,
        title: "Sodium",
        version: "0.6.5",
        iconUrl: null,
      },
      {
        name: "faithful-pack.zip",
        kind: "resourcePacks",
        enabled: false,
        size: 2048,
        title: "Faithful",
        version: null,
        iconUrl: null,
      },
    ]);

    renderContent("classic");

    expect(await screen.findByText("Sodium")).toBeInTheDocument();
    expect(screen.getByText("Faithful")).toBeInTheDocument();

    expect(
      screen.getByRole("columnheader", { name: "Content" }),
    ).toBeInTheDocument();
  });

  it("uses distinct same-size wrappers for content type icons", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "mod.jar",
        kind: "mods",
        enabled: true,
        size: 1024,
        title: "Mod",
        version: null,
        iconUrl: null,
      },
      {
        name: "pack.zip",
        kind: "resourcePacks",
        enabled: true,
        size: 1024,
        title: "Pack",
        version: null,
        iconUrl: null,
      },
      {
        name: "shader.zip",
        kind: "shaderPacks",
        enabled: true,
        size: 1024,
        title: "Shader",
        version: null,
        iconUrl: null,
      },
    ]);

    renderContent("classic");

    await screen.findByText("Mod");
    const wrappers = [
      ...document.querySelectorAll(
        ".size-8.shrink-0.items-center.justify-center.rounded-md",
      ),
    ];
    expect(wrappers).toHaveLength(3);
    expect(wrappers.map((wrapper) => wrapper.className)).toEqual(
      expect.arrayContaining([
        expect.stringContaining("bg-accent-violet/15"),
        expect.stringContaining("bg-accent-orange/15"),
        expect.stringContaining("bg-accent-teal/15"),
      ]),
    );
  });

  it("sorts content by name and size with direction indicators", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "zeta.jar",
        kind: "mods",
        enabled: true,
        size: 2048,
        title: "Zeta",
        version: null,
        iconUrl: null,
      },
      {
        name: "alpha.jar",
        kind: "mods",
        enabled: true,
        size: 1024,
        title: "Alpha",
        version: null,
        iconUrl: null,
      },
    ]);

    renderContent("classic");

    await screen.findByRole("button", { name: "Content" });
    let contentHeader = screen.getByRole("columnheader", { name: "Content" });
    let sizeHeader = screen.getByRole("columnheader", { name: "Size" });
    expect(contentHeader).toHaveAttribute("aria-sort", "ascending");
    expect(sizeHeader).toHaveAttribute("aria-sort", "none");

    fireEvent.click(screen.getByRole("button", { name: "Content" }));
    contentHeader = screen.getByRole("columnheader", { name: "Content" });
    expect(contentHeader).toHaveAttribute("aria-sort", "descending");

    fireEvent.click(screen.getByRole("button", { name: "Size" }));
    sizeHeader = screen.getByRole("columnheader", { name: "Size" });
    contentHeader = screen.getByRole("columnheader", { name: "Content" });
    expect(sizeHeader).toHaveAttribute("aria-sort", "ascending");
    expect(contentHeader).toHaveAttribute("aria-sort", "none");
  });

  it("uses the empty-page typography when no content is installed", async () => {
    listInstanceContent.mockResolvedValue([]);

    renderContent("classic");

    expect(await screen.findByText("No content installed.")).toHaveClass(
      "font-brand",
      "font-medium",
      "leading-normal",
    );
  });

  it("uses the same typography when filters have no matches", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "pack.zip",
        kind: "resourcePacks",
        enabled: true,
        size: 1024,
        title: "Pack",
        version: null,
        iconUrl: null,
      },
    ]);

    renderContent("classic", "mods");

    expect(
      await screen.findByText("No content matches your filters."),
    ).toHaveClass("font-brand", "font-medium", "leading-normal");
  });
});
