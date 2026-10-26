import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { StoragePanel } from "@/features/downloads/components/storage-panel";
import { storageSummary } from "@/features/downloads/fixtures/storage";
import { STORAGE_LOADING_PREVIEW_QUERY_KEY } from "@/features/downloads/hooks/use-storage-loading-preview";
import type { StorageSummary } from "@/lib/generated/StorageSummary";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function renderStorage(
  client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  }),
) {
  return render(
    <QueryClientProvider client={client}>
      <StoragePanel active={false} />
    </QueryClientProvider>,
  );
}

describe("StoragePanel", () => {
  beforeEach(() => invoke.mockReset());

  it("shows launcher consumption by category without exposing the disk name", async () => {
    invoke.mockResolvedValue(storageSummary);
    renderStorage();
    expect(
      screen.getByRole("heading", { name: "Launcher Data" }),
    ).toBeVisible();
    expect(screen.queryByText(/Loading/)).not.toBeInTheDocument();
    expect(screen.queryByText("20.0 GB")).not.toBeInTheDocument();
    expect(await screen.findByText("20.0 GB")).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Launcher Data" }),
    ).toBeVisible();
    expect(screen.queryByText(storageSummary.diskName)).not.toBeInTheDocument();
    expect(screen.queryByText(/Available|Used|Total/)).not.toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Launcher storage/ })).toBeVisible();
    const legend = within(
      screen.getByRole("list", { name: "Launcher storage breakdown" }),
    );
    for (const label of ["Instances", "Game files", "Java", "Cache", "Other"]) {
      expect(legend.getByText(label)).toBeVisible();
    }
    expect(legend.getByText("12.0 GB")).toBeVisible();
    expect(legend.getByText("750.0 MB")).toBeVisible();
    expect(screen.getByLabelText("Java segment")).toHaveClass(
      "fill-accent-violet",
    );
    expect(invoke).toHaveBeenCalledWith("storage_summary");
  });

  it("immediately reuses cached launcher consumption without a loading message", () => {
    invoke.mockResolvedValue(storageSummary);
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    client.setQueryData(["storage-summary"], storageSummary);
    renderStorage(client);
    expect(screen.getByText("20.0 GB")).toBeVisible();
    expect(screen.getByRole("img", { name: /Launcher storage/ })).toBeVisible();
    expect(screen.queryByText(/Loading/)).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows a neutral bar and a spinner beside every category until the data arrives", async () => {
    let resolveStorage: ((summary: StorageSummary) => void) | undefined;
    invoke.mockReturnValue(
      new Promise<StorageSummary>((resolve) => {
        resolveStorage = resolve;
      }),
    );
    renderStorage();
    expect(
      screen.getByRole("status", { name: "Loading launcher storage" }),
    ).toBeVisible();
    for (const category of [
      "Instances",
      "Game files",
      "Java",
      "Cache",
      "Other",
    ]) {
      expect(screen.getByText(category)).toBeVisible();
      expect(
        screen.getByRole("status", { name: `Loading ${category}` }),
      ).toBeVisible();
      expect(
        screen.getByRole("button", { name: `Loading ${category}` }),
      ).toBeDisabled();
    }
    expect(
      screen.getByRole("group", { name: "Launcher storage chart" }),
    ).toHaveAttribute("aria-busy", "true");
    expect(screen.queryByLabelText("Java segment")).not.toBeInTheDocument();
    await act(async () => resolveStorage?.(storageSummary));
    expect(await screen.findByText("20.0 GB")).toBeVisible();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Highlight Java: 2.0 GB" }),
    ).toBeEnabled();
  });

  it("previews loading for five seconds without replacing cached real measurements", async () => {
    vi.useFakeTimers();
    try {
      invoke.mockResolvedValue(storageSummary);
      const client = new QueryClient({
        defaultOptions: { queries: { retry: false } },
      });
      client.setQueryData(["storage-summary"], storageSummary);
      client.setQueryData(STORAGE_LOADING_PREVIEW_QUERY_KEY, Date.now());
      const view = renderStorage(client);
      expect(screen.getAllByRole("status")).toHaveLength(6);
      expect(screen.queryByText("20.0 GB")).not.toBeInTheDocument();
      expect(client.getQueryData(["storage-summary"])).toEqual(storageSummary);
      await act(async () => vi.advanceTimersByTimeAsync(4999));
      expect(screen.getAllByRole("status")).toHaveLength(6);
      await act(async () => vi.advanceTimersByTimeAsync(2));
      expect(screen.getByText("20.0 GB")).toBeVisible();
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
      expect(client.getQueryData(["storage-summary"])).toEqual(storageSummary);
      view.unmount();
    } finally {
      vi.useRealTimers();
    }
  });

  it("reports storage errors and retries without inventing launcher consumption", async () => {
    invoke
      .mockRejectedValueOnce(new Error("Disk unavailable"))
      .mockResolvedValue(storageSummary);
    renderStorage();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not load storage information.",
    );
    expect(screen.queryByText("20.0 GB")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("20.0 GB")).toBeVisible();
  });

  it.each([
    { bytes: 0, label: "0 B" },
    { bytes: 1500, label: "1.5 KB" },
    { bytes: 1_500_000, label: "1.5 MB" },
  ])(
    "formats $bytes bytes without implying disk capacity",
    async ({ bytes, label }) => {
      invoke.mockResolvedValue({
        ...storageSummary,
        managedBytes: bytes,
        categories: storageSummary.categories.map((usage, index) => ({
          ...usage,
          bytes: index === 0 ? bytes : 0,
        })),
      });
      renderStorage();
      await waitFor(() =>
        expect(
          screen.getByRole("heading", { name: "Launcher Data" }).parentElement,
        ).toHaveTextContent(label),
      );
    },
  );
});
