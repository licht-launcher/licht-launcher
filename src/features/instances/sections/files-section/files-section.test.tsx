import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  FilesBreadcrumb,
  FilesSection,
} from "@/features/instances/sections/files-section";
import type { InstanceFsEntry } from "@/lib/generated/InstanceFsEntry";

const listInstanceEntries = vi.hoisted(() => vi.fn());

vi.mock("@/lib/commands", () => ({
  listInstanceEntries,
}));

function FilesHarness({ folder }: { folder: string | null }) {
  const [relativePath, setRelativePath] = useState("");
  return (
    <>
      <FilesBreadcrumb
        folder={folder}
        relativePath={relativePath}
        onRelativePathChange={setRelativePath}
      />
      <FilesSection
        folder={folder}
        relativePath={relativePath}
        onRelativePathChange={setRelativePath}
      />
    </>
  );
}

function renderFiles(folder: string | null) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <FilesHarness folder={folder} />
    </QueryClientProvider>,
  );
}

describe("FilesSection", () => {
  beforeEach(() => {
    listInstanceEntries.mockReset();
  });

  it("asks to select an instance when none is selected", () => {
    renderFiles(null);
    expect(
      screen.getByText("Select an instance to browse its files."),
    ).toBeInTheDocument();
  });

  it("shows progress while loading", () => {
    listInstanceEntries.mockReturnValue(new Promise(() => {}));

    renderFiles("classic");

    expect(
      screen.getByRole("progressbar", { name: "Loading files" }),
    ).toBeInTheDocument();
  });

  it("sorts files by size in both directions while keeping folders first", async () => {
    listInstanceEntries.mockResolvedValue([
      { name: "saves", isDir: true, size: null, created: null },
      { name: "alpha.txt", isDir: false, size: 20, created: null },
      { name: "beta.txt", isDir: false, size: 10, created: null },
    ] satisfies InstanceFsEntry[]);

    renderFiles("classic");

    await screen.findByText("alpha.txt");
    const files = screen.getByLabelText("Instance files");
    const rowNames = () =>
      within(files)
        .getAllByLabelText(/, (file|folder)$/)
        .map((row) => row.getAttribute("aria-label"));
    expect(rowNames()).toEqual([
      "saves, folder",
      "alpha.txt, file",
      "beta.txt, file",
    ]);

    const sizeHeader = screen.getByRole("columnheader", { name: "Size" });
    const sortBySize = within(sizeHeader).getByRole("button", { name: "Size" });
    fireEvent.click(sortBySize);
    expect(sizeHeader).toHaveAttribute("aria-sort", "ascending");
    expect(rowNames()).toEqual([
      "saves, folder",
      "beta.txt, file",
      "alpha.txt, file",
    ]);

    fireEvent.click(sortBySize);
    expect(sizeHeader).toHaveAttribute("aria-sort", "descending");
    expect(rowNames()).toEqual([
      "saves, folder",
      "alpha.txt, file",
      "beta.txt, file",
    ]);
  });

  it("lists entries in a table and opens folders", async () => {
    const created = Date.UTC(2024, 0, 15, 12, 0, 0);
    const root: InstanceFsEntry[] = [
      { name: "saves", isDir: true, size: null, created },
      { name: "options.txt", isDir: false, size: 12, created },
    ];
    const saves: InstanceFsEntry[] = [
      { name: "world", isDir: true, size: null, created },
    ];
    listInstanceEntries.mockImplementation(
      async (_folder: string, relativePath: string) => {
        if (relativePath === "") {
          return root;
        }
        if (relativePath === "saves") {
          return saves;
        }
        return [];
      },
    );

    renderFiles("classic");

    expect(await screen.findByText("saves")).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "Name" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "Size" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "Created" }),
    ).toBeInTheDocument();
    expect(screen.getByText("options.txt")).toBeInTheDocument();
    expect(screen.getByText("12 B")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Up" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "saves, folder" }));
    await waitFor(() => {
      expect(listInstanceEntries).toHaveBeenCalledWith("classic", "saves");
    });
    expect(await screen.findByText("world")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "saves" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Up" })).toBeInTheDocument();
    expect(screen.getByText("..")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Up" }));
    await waitFor(() => {
      expect(listInstanceEntries).toHaveBeenCalledWith("classic", "");
    });
    expect(await screen.findByText("options.txt")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Up" }),
    ).not.toBeInTheDocument();
  });
});
