import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  classicInstance,
  invoke,
  mockContextMenuCommands,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

describe("Instance library", () => {
  setupInstancesPageTests();

  it("sorts instances by last played and by name in both directions", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_instances") {
        return Promise.resolve([
          {
            ...classicInstance,
            folder: "zeta",
            name: "Zeta",
            lastOpenedAt: "2026-01-01T10:00:00Z",
          },
          { ...classicInstance, lastOpenedAt: "2026-03-01T10:00:00Z" },
          { ...classicInstance, folder: "alpha", name: "Alpha" },
        ]);
      }
    });
    await renderAt("/?instance=classic");
    await screen.findByRole("heading", { name: "Classic" });
    const sidebar = within(
      screen.getByRole("navigation", { name: "Instances" }),
    );
    const names = () =>
      sidebar
        .getAllByRole("button")
        .map((button) => button.textContent)
        .filter((name) => ["Classic", "Alpha", "Zeta"].includes(name ?? ""));

    expect(names()).toEqual(["Classic", "Zeta", "Alpha"]);
    for (const [sort, expected] of [
      ["A–Z", ["Alpha", "Classic", "Zeta"]],
      ["Z–A", ["Zeta", "Classic", "Alpha"]],
      ["Last played", ["Classic", "Zeta", "Alpha"]],
    ] as const) {
      fireEvent.click(
        sidebar.getByRole("combobox", { name: "Sort instances" }),
      );
      fireEvent.click(await screen.findByRole("option", { name: sort }));
      expect(names()).toEqual(expected);
    }
  });

  it("preserves content filters when selecting another instance", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    fireEvent.change(screen.getByRole("textbox", { name: "Search content" }), {
      target: { value: "sodium" },
    });
    fireEvent.click(screen.getByRole("combobox", { name: "Filter content" }));
    fireEvent.click(await screen.findByRole("option", { name: "Mods" }));
    const sidebar = within(
      screen.getByRole("navigation", { name: "Instances" }),
    );
    for (const name of ["Next", "Classic"]) {
      fireEvent.click(sidebar.getByRole("button", { name }));
      expect(await screen.findByRole("heading", { name })).toBeVisible();
      expect(
        screen.getByRole("textbox", { name: "Search content" }),
      ).toHaveValue("sodium");
      expect(
        screen.getByRole("combobox", { name: "Filter content" }),
      ).toHaveTextContent("Mods");
    }
  });

  it("preserves filters, the file path and log levels across instance tabs", async () => {
    mockContextMenuCommands((command, args) => {
      if (command !== "list_instance_entries") return undefined;
      const relativePath =
        args !== null && typeof args === "object" && "relativePath" in args
          ? args.relativePath
          : undefined;
      return Promise.resolve(
        relativePath === "saves"
          ? [{ name: "readme.txt", isDir: false, size: 12, created: null }]
          : [{ name: "saves", isDir: true, size: null, created: null }],
      );
    });
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    fireEvent.change(screen.getByRole("textbox", { name: "Search content" }), {
      target: { value: "sodium" },
    });
    fireEvent.click(screen.getByRole("combobox", { name: "Filter content" }));
    fireEvent.click(await screen.findByRole("option", { name: "Mods" }));
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Files" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "saves, folder" }),
    );
    expect(await screen.findByText("readme.txt")).toBeVisible();
    const filesTab = screen.getByRole("tab", { name: "Files" });
    filesTab.focus();
    fireEvent.keyDown(filesTab, { key: "ArrowRight" });
    await screen.findByRole("tabpanel", { name: "Console" });
    fireEvent.click(screen.getByRole("button", { name: "Log level" }));
    fireEvent.click(await screen.findByRole("button", { name: "Warn" }));
    fireEvent.click(screen.getByRole("button", { name: "Log level" }));
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Content" }));
    expect(screen.getByRole("textbox", { name: "Search content" })).toHaveValue(
      "sodium",
    );
    expect(
      screen.getByRole("combobox", { name: "Filter content" }),
    ).toHaveTextContent("Mods");
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Files" }));
    expect(screen.getByRole("button", { name: "saves" })).toBeDisabled();
    expect(await screen.findByText("readme.txt")).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("list_instance_entries", {
      folder: "classic",
      relativePath: "saves",
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(screen.getByRole("button", { name: "Log level" })).toHaveTextContent(
      "Warn",
    );
  });

  it("guides an empty library to the creation wizard without instance tabs", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return Promise.resolve([]);
    });
    const { router } = await renderAt("/");
    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Choose a Minecraft version or install a modpack to start playing.",
      ),
    ).toBeVisible();
    expect(screen.queryByText("No instances yet.")).not.toBeInTheDocument();
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("textbox", { name: "Search content" }),
    ).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Create instance" }));
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
  });

  it("waits for the instance list before showing the welcome state", async () => {
    let finishLoading: (entries: InstanceEntry[]) => void = () => {};
    const pendingInstances = new Promise<InstanceEntry[]>((resolve) => {
      finishLoading = resolve;
    });
    mockContextMenuCommands((command) => {
      if (command === "list_instances") return pendingInstances;
    });
    await renderAt("/");
    expect(
      screen.queryByRole("heading", { name: "Create your first instance" }),
    ).not.toBeInTheDocument();
    await act(async () => finishLoading([]));
    expect(
      await screen.findByRole("heading", {
        name: "Create your first instance",
      }),
    ).toBeVisible();
  });

  it("opens the creator from the footer and restores selection through Library and history", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/?instance=next");
    await screen.findByRole("heading", { name: "Next" });
    fireEvent.click(screen.getByRole("link", { name: "Create instance" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
    expect(router.state.location.search).toMatchObject({ step: "source" });
    expect(
      invoke.mock.calls.some(
        ([command]) => command === "create_instance_with_platform",
      ),
    ).toBe(false);
    const library = screen.getByRole("button", { name: "Library" });
    expect(library).not.toHaveAttribute("aria-current");
    fireEvent.click(library);
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
    expect(library).toHaveAttribute("aria-current", "page");
    expect(router.state.location.search).toMatchObject({ instance: "next" });
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("heading", { name: "How do you want to start?" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
  });

  it("preserves the Downloads route and restores the library sidebar on return", async () => {
    mockContextMenuCommands();
    const { router } = await renderAt("/downloads");
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(router.state.location.pathname).toBe("/downloads");
    expect(
      screen.queryByRole("navigation", { name: "Instances" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("separator", { name: "Resize sidebar" }),
    ).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Library" }));
      await router.load();
    });
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    expect(screen.getByRole("navigation", { name: "Instances" })).toBeVisible();
    fireEvent.click(
      await screen.findByRole("button", { name: "Downloads: Idle" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Downloads" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("navigation", { name: "Instances" }),
    ).not.toBeInTheDocument();
  });

  it("treats instance switches as history entries", async () => {
    mockContextMenuCommands();
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
    fireEvent.click(
      within(screen.getByRole("navigation", { name: "Instances" })).getByRole(
        "button",
        { name: "Next" },
      ),
    );
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(
      await screen.findByRole("heading", { name: "Classic" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(await screen.findByRole("heading", { name: "Next" })).toBeVisible();
  });

  it.each([
    { instance: "next", folder: "next", name: "Next" },
    { instance: "missing", folder: "classic", name: "Classic" },
  ])("selects $folder when opening ?instance=$instance", async (entry) => {
    mockContextMenuCommands();
    const { router } = await renderAt(`/?instance=${entry.instance}`);
    expect(
      await screen.findByRole("heading", { name: entry.name }),
    ).toBeVisible();
    await waitFor(() =>
      expect(router.state.location.search).toMatchObject({
        instance: entry.folder,
      }),
    );
    expect(
      within(screen.getByRole("navigation", { name: "Instances" })).getByRole(
        "button",
        { name: entry.name },
      ),
    ).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
  });

  it("reports a failed instance query", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_instances")
        return Promise.reject(new Error("HTTP request failed"));
    });
    await renderAt("/");
    expect(
      await screen.findByText(
        "The network request failed. Check your connection and try again.",
      ),
    ).toBeVisible();
  });
});
