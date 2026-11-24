import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { instanceContentQuery } from "@/features/instances/lib/content-query";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { ContentSection } from "@/features/instances/sections/content-section";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

const listInstanceContent = vi.hoisted(() => vi.fn());
const originalScrollTo = Object.getOwnPropertyDescriptor(
  HTMLElement.prototype,
  "scrollTo",
);

vi.mock("@/api/commands", () => ({
  listInstanceContent,
  removeInstanceContent: vi.fn(),
  setInstanceContentEnabled: vi.fn(),
  getModpack: vi.fn(),
  listModpackVersions: vi.fn(),
}));

function renderContent(
  folder: string | null,
  filter: "all" | "mods" | "resourcePacks" | "shaderPacks" = "all",
  search = "",
  modpack: ModpackOrigin | null = null,
) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={client}>
      <ContentSection
        folder={folder}
        filter={filter}
        search={search}
        modpack={modpack}
        toolbar={<button type="button">Refresh content</button>}
      />
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

  it("shows titles from different content types in the combined list", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "sodium-fabric-1.0.jar",
        kind: "mods",
        enabled: true,
        size: 1024,
        title: "Sodium",
        version: "0.6.5",
        iconUrl: null,
        fromModpack: false,
      },
      {
        name: "faithful-pack.zip",
        kind: "resourcePacks",
        enabled: false,
        size: 2048,
        title: "Faithful",
        version: null,
        iconUrl: null,
        fromModpack: false,
      },
    ]);

    renderContent("classic");

    expect(await screen.findByText("Sodium")).toBeInTheDocument();
    expect(screen.getByText("Faithful")).toBeInTheDocument();

    expect(
      screen.getByRole("columnheader", { name: "Content" }),
    ).toBeInTheDocument();
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
      fromModpack: false,
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
        fromModpack: false,
      },
      {
        name: "zeta.jar",
        kind: "mods",
        enabled: true,
        size: 4,
        title: "Zeta",
        version: null,
        iconUrl: null,
        fromModpack: false,
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
      fromModpack: false,
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

  it("sorts content by name with a direction indicator", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "zeta.jar",
        kind: "mods",
        enabled: true,
        size: 2048,
        title: "Zeta",
        version: null,
        iconUrl: null,
        fromModpack: false,
      },
      {
        name: "alpha.jar",
        kind: "mods",
        enabled: true,
        size: 1024,
        title: "Alpha",
        version: null,
        iconUrl: null,
        fromModpack: false,
      },
    ]);

    renderContent("classic");

    await screen.findByRole("button", { name: "Content" });
    let contentHeader = screen.getByRole("columnheader", { name: "Content" });
    expect(contentHeader).toHaveAttribute("aria-sort", "ascending");
    expect(
      screen.queryByRole("columnheader", { name: "Size" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Content" }));
    contentHeader = screen.getByRole("columnheader", { name: "Content" });
    expect(contentHeader).toHaveAttribute("aria-sort", "descending");
  });

  it("shows the empty installation message when no content is installed", async () => {
    listInstanceContent.mockResolvedValue([]);

    renderContent("classic");

    expect(await screen.findByText("No content installed.")).toBeVisible();
    expect(
      screen.queryByText("No content matches your filters."),
    ).not.toBeInTheDocument();
  });

  it("shows the no-matches message when installed content is filtered out", async () => {
    listInstanceContent.mockResolvedValue([
      {
        name: "pack.zip",
        kind: "resourcePacks",
        enabled: true,
        size: 1024,
        title: "Pack",
        version: null,
        iconUrl: null,
        fromModpack: false,
      },
    ]);

    renderContent("classic", "mods");

    expect(
      await screen.findByText("No content matches your filters."),
    ).toBeVisible();
    expect(screen.queryByText("No content installed.")).not.toBeInTheDocument();
    expect(screen.queryByText("Pack")).not.toBeInTheDocument();
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
      fromModpack: false,
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
    fireEvent.click(screen.getByRole("button", { name: "Content" }));
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

  describe("with a modpack", () => {
    const imported: ModpackOrigin = { type: "modpackFile", source: "modrinth" };
    const packed = (
      title: string,
      fromModpack: boolean,
    ): InstanceContentEntry => ({
      name: `${title.toLowerCase()}.jar`,
      kind: "mods",
      enabled: true,
      size: 1,
      title,
      version: null,
      iconUrl: null,
      fromModpack,
    });

    it("lists only the content added on top of the modpack", async () => {
      listInstanceContent.mockResolvedValue([
        packed("Sodium", true),
        packed("Extra", false),
      ]);

      renderContent("classic", "all", "", imported);

      expect(await screen.findByText("Extra")).toBeInTheDocument();
      expect(screen.queryByText("Sodium")).not.toBeInTheDocument();
      expect(
        screen.getByRole("region", { name: "Modpack content" }),
      ).toHaveTextContent("1 mod");
      expect(
        screen.getByRole("heading", { name: "Extra content 1" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "Refresh content" }),
      ).toBeInTheDocument();
    });

    it("says when nothing was added on top of the modpack", async () => {
      listInstanceContent.mockResolvedValue([packed("Sodium", true)]);

      renderContent("classic", "all", "", imported);

      expect(
        await screen.findByRole("status", {
          name: "No extra content installed",
        }),
      ).toBeInTheDocument();
      expect(screen.queryByRole("columnheader")).not.toBeInTheDocument();
      expect(
        screen.queryByRole("heading", { name: /Extra content/ }),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: "Refresh content" }),
      ).not.toBeInTheDocument();
    });

    it("opens the modpack content in a dialog and closes it again", async () => {
      listInstanceContent.mockResolvedValue([
        packed("Sodium", true),
        packed("Extra", false),
      ]);
      renderContent("classic", "all", "", imported);
      await screen.findByText("Extra");

      fireEvent.click(screen.getByRole("button", { name: "View content" }));

      const dialog = await screen.findByRole("dialog", {
        name: "Modpack content",
      });
      expect(within(dialog).getByText("Sodium")).toBeInTheDocument();
      expect(within(dialog).queryByText("Extra")).not.toBeInTheDocument();

      fireEvent.click(within(dialog).getByRole("button", { name: "Close" }));

      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      expect(screen.getByText("Extra")).toBeInTheDocument();
    });

    it("lists everything without a modpack card for other instances", async () => {
      listInstanceContent.mockResolvedValue([packed("Extra", false)]);

      renderContent("classic");

      expect(await screen.findByText("Extra")).toBeInTheDocument();
      expect(
        screen.queryByRole("region", { name: "Modpack content" }),
      ).not.toBeInTheDocument();
    });
  });
});
