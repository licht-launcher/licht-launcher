import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { instanceContentQuery } from "@/features/instances/lib/content-query";
import { ContentSection } from "@/features/instances/sections/content-section";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

const listInstanceContent = vi.hoisted(() => vi.fn());
const originalScrollTo = Object.getOwnPropertyDescriptor(
  HTMLElement.prototype,
  "scrollTo",
);

vi.mock("@/lib/commands", () => ({
  listInstanceContent,
  removeInstanceContent: vi.fn(),
  setInstanceContentEnabled: vi.fn(),
}));

function renderContent(
  folder: string | null,
  filter: "all" | "mods" | "resourcePacks" | "shaderPacks" = "all",
  search = "",
) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={client}>
      <ContentSection folder={folder} filter={filter} search={search} />
    </QueryClientProvider>,
  );
  return { ...view, client };
}

describe("ContentSection", () => {
  beforeEach(() => {
    listInstanceContent.mockReset();
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        if (this.dataset.slot === "scroll-area-viewport") return 260;
        return this.tagName === "TR" && this.hasAttribute("data-index")
          ? 52
          : 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(960);
    vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        return this.dataset.slot === "scroll-area-viewport" ? 260 : 0;
      },
    );
    vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(
      function (this: HTMLElement) {
        return [
          ...this.querySelectorAll<HTMLTableRowElement>("tbody > tr"),
        ].reduce(
          (total, row) =>
            total +
            (row.hasAttribute("data-index")
              ? row.offsetHeight
              : Number.parseFloat(row.cells[0]?.style.height || "0")),
          0,
        );
      },
    );
    Object.defineProperty(HTMLElement.prototype, "scrollTo", {
      configurable: true,
      value: function (this: HTMLElement, options: ScrollToOptions) {
        const offset = Math.max(
          0,
          Math.min(
            options.top ?? this.scrollTop,
            this.scrollHeight - this.clientHeight,
          ),
        );
        if (offset !== this.scrollTop) {
          this.scrollTop = offset;
          queueMicrotask(() => this.dispatchEvent(new Event("scroll")));
        }
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    if (originalScrollTo)
      Object.defineProperty(
        HTMLElement.prototype,
        "scrollTo",
        originalScrollTo,
      );
    else Reflect.deleteProperty(HTMLElement.prototype, "scrollTo");
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

  it("shows progress during refresh while keeping cached content visible", async () => {
    const entry: InstanceContentEntry = {
      name: "sodium.jar",
      kind: "mods",
      enabled: true,
      size: 1024,
      title: "Sodium",
      version: "0.6.5",
      iconUrl: null,
    };
    let completeRefresh: (entries: InstanceContentEntry[]) => void = () => {};
    listInstanceContent.mockResolvedValueOnce([entry]).mockImplementationOnce(
      () =>
        new Promise<InstanceContentEntry[]>((resolve) => {
          completeRefresh = resolve;
        }),
    );
    const { client } = renderContent("classic");
    expect(await screen.findByText("Sodium")).toBeVisible();
    expect(
      screen.queryByRole("progressbar", { name: "Loading content metadata" }),
    ).not.toBeInTheDocument();

    act(() => {
      void client.fetchQuery({
        ...instanceContentQuery(client, "classic", true),
        staleTime: 0,
      });
    });

    expect(
      await screen.findByRole("progressbar", {
        name: "Loading content metadata",
      }),
    ).toBeVisible();
    expect(screen.getByText("Sodium")).toBeVisible();
    expect(
      screen.getByRole("switch", { name: "Disable Sodium" }),
    ).toBeVisible();

    await act(async () => {
      completeRefresh([{ ...entry, version: "0.6.6" }]);
    });
    expect(await screen.findByText("0.6.6 · sodium.jar")).toBeVisible();
    await waitFor(() => {
      expect(
        screen.queryByRole("progressbar", { name: "Loading content metadata" }),
      ).not.toBeInTheDocument();
    });
  });

  it("shows and sorts local rows while metadata is pending, then applies enriched batches", async () => {
    const entries: InstanceContentEntry[] = [
      {
        name: "alpha.jar",
        kind: "mods",
        enabled: true,
        size: 3,
        title: "Alpha",
        version: null,
        iconUrl: null,
      },
      {
        name: "zeta.jar",
        kind: "mods",
        enabled: true,
        size: 4,
        title: "Zeta",
        version: null,
        iconUrl: null,
      },
    ];
    let publish:
      | ((entries: InstanceContentEntry[], initial: boolean) => void)
      | undefined;
    let finish: ((entries: InstanceContentEntry[]) => void) | undefined;
    listInstanceContent.mockImplementation((_folder, _refresh, onUpdate) => {
      publish = onUpdate;
      return new Promise<InstanceContentEntry[]>((resolve) => {
        finish = resolve;
      });
    });
    const { client } = renderContent("classic");
    await waitFor(() => expect(publish).toBeDefined());
    act(() => publish?.(entries, true));
    expect(await screen.findByText("Alpha")).toBeVisible();
    expect(screen.getByText("Zeta")).toBeVisible();
    expect(
      screen.getByRole("progressbar", { name: "Loading content metadata" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Content" }));
    expect(screen.getAllByRole("switch")[0]).toHaveAccessibleName(
      "Disable Zeta",
    );
    const enriched = { ...entries[1], title: "Resolved Zeta", version: "1.0" };
    act(() => publish?.([enriched], false));
    expect(await screen.findByText("Resolved Zeta")).toBeVisible();
    expect(screen.getByText("Alpha")).toBeVisible();
    expect(client.getQueryData(["instance-content", "classic"])).toEqual([
      entries[0],
      enriched,
    ]);
    await act(async () => finish?.([entries[0], enriched]));
    await waitFor(() =>
      expect(
        screen.queryByRole("progressbar", { name: "Loading content metadata" }),
      ).not.toBeInTheDocument(),
    );
  });

  it("preserves partial content and reports a metadata failure", async () => {
    const entry: InstanceContentEntry = {
      name: "one.jar",
      kind: "mods",
      enabled: true,
      size: 3,
      title: "One",
      version: null,
      iconUrl: null,
    };
    let publish:
      | ((entries: InstanceContentEntry[], initial: boolean) => void)
      | undefined;
    let fail: ((reason: Error) => void) | undefined;
    listInstanceContent.mockImplementation((_folder, _refresh, onUpdate) => {
      publish = onUpdate;
      return new Promise<InstanceContentEntry[]>((_resolve, reject) => {
        fail = reject;
      });
    });
    renderContent("classic");
    await waitFor(() => expect(publish).toBeDefined());
    act(() => publish?.([entry], true));
    expect(await screen.findByText("One")).toBeVisible();
    await act(async () => fail?.(new Error("metadata failed")));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Some content metadata could not be loaded.",
    );
    expect(screen.getByText("One")).toBeVisible();
    expect(
      screen.queryByRole("progressbar", { name: "Loading content metadata" }),
    ).not.toBeInTheDocument();
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

  function largeContent(): InstanceContentEntry[] {
    return Array.from({ length: 3000 }, (_, index) => ({
      name: `mod-${String(index).padStart(4, "0")}.jar`,
      kind: "mods",
      enabled: true,
      size: index + 1,
      title: `Mod ${String(index).padStart(4, "0")}`,
      version: null,
      iconUrl: null,
    }));
  }

  it("keeps a large list bounded in the DOM and renders distant rows on scroll", async () => {
    listInstanceContent.mockResolvedValue(largeContent());
    renderContent("classic");
    expect(await screen.findByText("Mod 0000")).toBeVisible();
    expect(
      screen.getByRole("table", { name: "Installed content" }),
    ).toHaveAttribute("aria-rowcount", "3000");
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
    expect(screen.queryByText("Mod 1500")).not.toBeInTheDocument();
    const viewport = document.querySelector<HTMLDivElement>(
      "[data-slot=scroll-area-viewport]",
    );
    if (!viewport) throw new Error("missing viewport");
    fireEvent.scroll(viewport, { target: { scrollTop: 1500 * 56 } });
    await waitFor(() =>
      expect(document.querySelector("tr[data-index='1500']")).not.toBeNull(),
    );
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
    expect(screen.queryByText("Mod 0000")).not.toBeInTheDocument();
  });

  it("searches all entries and resets a scrolled list when filters or sorting change", async () => {
    const entries = largeContent();
    entries[2999] = { ...entries[2999], kind: "resourcePacks" };
    listInstanceContent.mockResolvedValue(entries);
    const { client, rerender } = renderContent("classic");
    await screen.findByText("Mod 0000");
    const viewport = document.querySelector<HTMLDivElement>(
      "[data-slot=scroll-area-viewport]",
    );
    if (!viewport) throw new Error("missing viewport");
    fireEvent.scroll(viewport, { target: { scrollTop: 1500 * 56 } });
    rerender(
      <QueryClientProvider client={client}>
        <ContentSection folder="classic" filter="all" search="2999" />
      </QueryClientProvider>,
    );
    expect(await screen.findByText("Mod 2999")).toBeVisible();
    expect(viewport.scrollTop).toBe(0);
    expect(screen.getAllByRole("switch")).toHaveLength(1);
    rerender(
      <QueryClientProvider client={client}>
        <ContentSection folder="classic" filter="resourcePacks" search="" />
      </QueryClientProvider>,
    );
    expect(screen.getByText("Mod 2999")).toBeVisible();
    expect(screen.getAllByRole("switch")).toHaveLength(1);
    rerender(
      <QueryClientProvider client={client}>
        <ContentSection folder="classic" filter="all" search="" />
      </QueryClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Size" }));
    fireEvent.click(screen.getByRole("button", { name: "Size" }));
    expect(await screen.findByText("Mod 2999")).toBeVisible();
    expect(screen.getAllByRole("switch")[0]).toHaveAccessibleName(
      "Disable Mod 2999",
    );
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
  });

  it("keeps an offscreen focused row mounted and tabs to unmounted neighboring controls", async () => {
    listInstanceContent.mockResolvedValue(largeContent());
    renderContent("classic");
    const firstRemove = await screen.findByRole("button", {
      name: "Remove Mod 0000",
    });
    act(() => firstRemove.focus());
    const viewport = document.querySelector<HTMLDivElement>(
      "[data-slot=scroll-area-viewport]",
    );
    if (!viewport) throw new Error("missing viewport");
    fireEvent.scroll(viewport, { target: { scrollTop: 1500 * 56 } });
    await waitFor(() =>
      expect(document.querySelector("tr[data-index='1500']")).not.toBeNull(),
    );
    expect(firstRemove).toHaveFocus();
    expect(
      screen.queryByRole("switch", { name: "Disable Mod 0001" }),
    ).not.toBeInTheDocument();
    fireEvent.keyDown(firstRemove, { key: "Tab" });
    const nextToggle = await screen.findByRole("switch", {
      name: "Disable Mod 0001",
    });
    await waitFor(() => expect(nextToggle).toHaveFocus());
    expect(viewport.scrollTop).toBeLessThan(260);
    fireEvent.keyDown(nextToggle, { key: "Tab", shiftKey: true });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Remove Mod 0000" }),
      ).toHaveFocus(),
    );
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
  });

  it("updates a distant row from progressive data without mounting the entire list", async () => {
    const entries = largeContent();
    listInstanceContent.mockResolvedValue(entries);
    const { client } = renderContent("classic");
    await screen.findByText("Mod 0000");
    act(() =>
      client.setQueryData(
        ["instance-content", "classic"],
        entries.map((entry, index) =>
          index === 2999 ? { ...entry, version: "2.0" } : entry,
        ),
      ),
    );
    const viewport = document.querySelector<HTMLDivElement>(
      "[data-slot=scroll-area-viewport]",
    );
    if (!viewport) throw new Error("missing viewport");
    fireEvent.scroll(viewport, { target: { scrollTop: 3000 * 56 } });
    expect(await screen.findByText("2.0 · mod-2999.jar")).toBeVisible();
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
  });

  it("keeps the dialog trigger mounted and restores focus when removal is cancelled", async () => {
    listInstanceContent.mockResolvedValue(largeContent());
    renderContent("classic");
    const trigger = await screen.findByRole("button", {
      name: "Remove Mod 0000",
    });
    act(() => trigger.focus());
    fireEvent.click(trigger);
    await screen.findByRole("dialog");
    const viewport = document.querySelector<HTMLDivElement>(
      "[data-slot=scroll-area-viewport]",
    );
    if (!viewport) throw new Error("missing viewport");
    fireEvent.scroll(viewport, { target: { scrollTop: 1500 * 56 } });
    await waitFor(() =>
      expect(document.querySelector("tr[data-index='1500']")).not.toBeNull(),
    );
    expect(trigger).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(screen.getAllByRole("switch").length).toBeLessThan(30);
  });
});
