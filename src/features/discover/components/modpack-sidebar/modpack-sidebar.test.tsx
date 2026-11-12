import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { modpackDetails } from "@/features/discover/fixtures/modpacks";

import { ModpackSidebar } from "./modpack-sidebar";

const { openExternalUrl } = vi.hoisted(() => ({
  openExternalUrl: vi.fn<(url: string) => Promise<void>>(),
}));

vi.mock("@/api/commands", () => ({ openExternalUrl }));

describe("ModpackSidebar", () => {
  it("lists compatible game versions and loaders", () => {
    render(<ModpackSidebar modpack={modpackDetails()} />);

    const versions = screen.getByRole("list", { name: "Minecraft versions" });
    expect(within(versions).getAllByRole("listitem")).toHaveLength(2);
    expect(versions).toHaveTextContent("1.21.10");
    const loaders = screen.getByRole("list", { name: "Loaders" });
    expect(within(loaders).getByText("Fabric")).toHaveAttribute(
      "data-loader",
      "fabric",
    );
  });

  it("opens only the links the pack has in the system browser", () => {
    openExternalUrl.mockResolvedValue();
    render(<ModpackSidebar modpack={modpackDetails()} />);

    expect(
      screen.queryByRole("button", { name: "Discord" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Source code" }));

    expect(openExternalUrl).toHaveBeenCalledWith(
      "https://github.com/Fabulously-Optimized/fabulously-optimized",
    );
  });

  it("shows the creator, license and dates", () => {
    openExternalUrl.mockResolvedValue();
    render(<ModpackSidebar modpack={modpackDetails()} />);

    expect(screen.getByText("robotkoer")).toBeInTheDocument();
    expect(screen.getByText("Feb 10, 2022")).toBeInTheDocument();
    expect(screen.getByText("Nov 1, 2026")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "BSD 3-Clause" }));
    expect(openExternalUrl).toHaveBeenCalledWith(
      "https://github.com/Fabulously-Optimized/fabulously-optimized/blob/main/LICENSE.md",
    );
  });

  it("leaves out sections the source sent nothing for", () => {
    render(
      <ModpackSidebar
        modpack={modpackDetails({
          gameVersions: [],
          loaders: [],
          author: "",
          license: { name: "All rights reserved", url: null },
          links: { source: null, issues: null, wiki: null, discord: null },
        })}
      />,
    );

    expect(screen.queryByText("Compatibility")).not.toBeInTheDocument();
    expect(screen.queryByText("Links")).not.toBeInTheDocument();
    expect(screen.queryByText("Creators")).not.toBeInTheDocument();
    expect(screen.getByText("All rights reserved")).not.toHaveRole("button");
  });
});
