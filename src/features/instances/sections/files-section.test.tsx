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

    expect(
      await screen.findByRole("columnheader", { name: "Name" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "Size" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("columnheader", { name: "Created" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Instance files")).toHaveClass(
      "border-border/60",
      "bg-black/20",
    );
    expect(screen.getByText("saves")).toBeInTheDocument();
    expect(screen.getByText("options.txt")).toBeInTheDocument();
    expect(screen.getByText("12 B")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    const folderRow = screen.getByRole("button", { name: "saves, folder" });
    expect(folderRow.querySelector("svg")).toHaveClass("text-muted-foreground");
    const fileRow = screen.getByLabelText("options.txt, file");
    expect(fileRow.querySelector("svg")).toHaveClass("text-muted-foreground");

    fireEvent.click(
      within(screen.getByRole("columnheader", { name: /Size/ })).getByRole(
        "button",
      ),
    );
    await waitFor(() => {
      const bodyRows = [
        ...screen.getByLabelText("Instance files").querySelectorAll("tbody tr"),
      ].map((row) => row.textContent ?? "");
      expect(bodyRows[0]).toContain("saves");
      expect(bodyRows[1]).toContain("options.txt");
    });

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
