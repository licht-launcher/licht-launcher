import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import {
  classicInstance,
  invoke,
  listeners,
  mockContextMenuCommands,
  openInstanceContextMenu,
  renderAt,
  setupInstancesPageTests,
} from "@/features/instances/tests/support/render-instances";
import type { GameLogEvent } from "@/lib/generated/GameLogEvent";

describe("Instance actions", () => {
  setupInstancesPageTests();

  beforeEach(() => {
    mockContextMenuCommands();
  });

  it("targets the clicked instance without changing selection or opening the global menu", async () => {
    const { router } = await renderAt("/?instance=classic");
    const menu = await openInstanceContextMenu();
    expect(screen.getAllByRole("menu")).toHaveLength(1);
    expect(within(menu).queryByText("Next")).not.toBeInTheDocument();
    expect(
      within(menu).queryByRole("menuitem", { name: "Settings" }),
    ).not.toBeInTheDocument();
    expect(router.state.location.search).toMatchObject({
      instance: "classic",
    });
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() => {
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
    expect(screen.getByRole("heading", { name: "Classic" })).toBeVisible();
  });

  it("opens from the keyboard and restores focus when dismissed", async () => {
    await renderAt("/?instance=classic");
    const sidebar = await screen.findByRole("navigation", {
      name: "Instances",
    });
    const target = within(sidebar).getByRole("button", { name: "Next" });
    target.focus();
    fireEvent.keyDown(target, { key: "F10", shiftKey: true });
    const menu = await screen.findByRole("menu", {
      name: "Actions for Next",
    });
    expect(screen.getAllByRole("menu")).toHaveLength(1);
    fireEvent.keyDown(menu, { key: "Escape" });
    await waitFor(() => expect(target).toHaveFocus());
  });

  it("plays and stops the clicked instance and reflects transitional states", async () => {
    let finishLaunch: (exit: { code: number }) => void = () => {};
    let finishStop: () => void = () => {};
    mockContextMenuCommands((command) => {
      if (command === "launch") {
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch = resolve;
        });
      }
      if (command === "stop_game") {
        return new Promise<void>((resolve) => {
          finishStop = resolve;
        });
      }
      return undefined;
    });
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Play" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "next",
        username: "Steve",
      });
    });
    let menu = await openInstanceContextMenu();
    expect(
      within(menu).getByRole("menuitem", { name: "Starting…" }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(
      within(menu).getByRole("menuitem", { name: "Delete" }),
    ).toHaveAttribute("aria-disabled", "true");
    fireEvent.keyDown(menu, { key: "Escape" });
    act(() => {
      listeners.get("game-log")?.({
        payload: {
          folder: "next",
          stream: "stdout",
          line: "Started",
        } satisfies GameLogEvent,
      });
    });
    menu = await openInstanceContextMenu();
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Stop" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "next" });
    });
    menu = await openInstanceContextMenu();
    expect(
      within(menu).getByRole("menuitem", { name: "Stopping…" }),
    ).toHaveAttribute("aria-disabled", "true");
    await act(async () => {
      finishStop();
      finishLaunch({ code: 0 });
    });
    await waitFor(() => {
      expect(
        within(menu).getByRole("menuitem", { name: "Play" }),
      ).not.toHaveAttribute("aria-disabled", "true");
    });
  });

  it("opens the clicked instance folder", async () => {
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Open folder" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("open_instance_folder", {
        folder: "next",
      });
    });
  });

  it("opens configuration for the clicked instance", async () => {
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Configure" }));
    expect(await screen.findByLabelText("Name")).toHaveValue("Next");
    expect(screen.getByRole("dialog")).toBeVisible();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("renames the clicked instance through the action dialog", async () => {
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    const dialog = await screen.findByRole("dialog", { name: "Rename Next" });
    const field = within(dialog).getByLabelText("Name");
    const save = within(dialog).getByRole("button", {
      name: "Rename instance",
    });
    expect(field).toHaveValue("Next");
    expect(save).toBeDisabled();
    fireEvent.change(field, { target: { value: "   " } });
    expect(save).toBeDisabled();
    fireEvent.change(field, { target: { value: " Renamed " } });
    fireEvent.click(save);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("rename_instance", {
        folder: "next",
        name: "Renamed",
      });
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    expect(
      await within(sidebar).findByRole("button", { name: "Renamed" }),
    ).toBeVisible();
  });

  it("duplicates the clicked instance after entering a name", async () => {
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));
    const dialog = await screen.findByRole("dialog", {
      name: "Duplicate Next",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "duplicate_instance",
      expect.anything(),
    );
    fireEvent.change(within(dialog).getByLabelText("Name"), {
      target: { value: "Next copy" },
    });
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Duplicate instance" }),
    );
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("duplicate_instance", {
        folder: "next",
        name: "Next copy",
      });
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    expect(
      await screen.findByRole("button", { name: "Next copy" }),
    ).toBeVisible();
  });

  it("deletes only the clicked instance after confirmation", async () => {
    await renderAt("/?instance=classic");
    await openInstanceContextMenu();
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    const dialog = await screen.findByRole("dialog", {
      name: "Delete Next?",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "delete_instance",
      expect.anything(),
    );
    fireEvent.click(
      within(dialog).getByRole("button", { name: "Delete instance" }),
    );
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("delete_instance", {
        folder: "next",
      });
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    const sidebar = screen.getByRole("navigation", { name: "Instances" });
    await waitFor(() => {
      expect(
        within(sidebar).queryByRole("button", { name: "Next" }),
      ).not.toBeInTheDocument();
    });
    expect(
      within(sidebar).getByRole("button", { name: "Classic" }),
    ).toBeVisible();
  });

  it("launches an installed instance, streams its log and stops the game", async () => {
    let finishLaunch: (exit: { code: number }) => void = () => {};
    mockContextMenuCommands((command) => {
      if (command === "launch")
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch = resolve;
        });
    });
    await renderAt("/");
    fireEvent.click(await screen.findByRole("button", { name: "Play" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("launch", {
        folder: "classic",
        username: "Steve",
      }),
    );
    expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();
    expect(invoke).not.toHaveBeenCalledWith(
      "prepare_instance_download",
      expect.anything(),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "install_modpack_files",
      expect.anything(),
    );
    act(() =>
      listeners.get("game-log")?.({
        payload: {
          folder: "classic",
          stream: "stdout",
          line: "Sound engine started",
        } satisfies GameLogEvent,
      }),
    );
    expect(await screen.findByRole("button", { name: "Stop" })).toBeEnabled();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(await screen.findByText("Sound engine started")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(
      await screen.findByRole("button", { name: "Stopping…" }),
    ).toBeDisabled();
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" }),
    );
    await act(async () => finishLaunch({ code: 0 }));
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
  });

  it("recovers a running game on load and allows stopping it", async () => {
    mockContextMenuCommands((command) => {
      if (command === "list_running_games") return Promise.resolve(["classic"]);
    });
    await renderAt("/");
    expect(await screen.findByRole("button", { name: "Stop" })).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "Play" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Classic (running)" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("stop_game", { folder: "classic" }),
    );
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Classic" })).toBeVisible();
  });

  it("keeps console output isolated while two instances run", async () => {
    const finishLaunch: Record<string, (exit: { code: number }) => void> = {};
    mockContextMenuCommands((command, args) => {
      if (
        command === "launch" &&
        args !== null &&
        typeof args === "object" &&
        "folder" in args &&
        typeof args.folder === "string"
      ) {
        const folder = args.folder;
        return new Promise<{ code: number }>((resolve) => {
          finishLaunch[folder] = resolve;
        });
      }
    });
    await renderAt("/");
    await screen.findByRole("heading", { name: "Classic" });
    const sidebar = within(
      screen.getByRole("navigation", { name: "Instances" }),
    );
    for (const [name, folder] of [
      ["Classic", classicInstance.folder],
      ["Next", "next"],
    ]) {
      fireEvent.click(sidebar.getByRole("button", { name }));
      fireEvent.click(await screen.findByRole("button", { name: "Play" }));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith("launch", {
          folder,
          username: "Steve",
        }),
      );
      expect(screen.getByRole("button", { name: "Starting…" })).toBeDisabled();
    }
    act(() => {
      for (const folder of ["classic", "next"]) {
        listeners.get("game-log")?.({
          payload: {
            folder,
            stream: "stdout",
            line: `${folder}-log`,
          } satisfies GameLogEvent,
        });
      }
    });
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(await screen.findByText("next-log")).toBeVisible();
    expect(screen.queryByText("classic-log")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Clear console" }));
    expect(screen.queryByText("next-log")).not.toBeInTheDocument();
    fireEvent.click(sidebar.getByRole("button", { name: /^Classic/ }));
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Console" }));
    expect(await screen.findByText("classic-log")).toBeVisible();
    expect(screen.queryByText("next-log")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
    await act(async () => {
      finishLaunch.classic?.({ code: 0 });
      finishLaunch.next?.({ code: 0 });
    });
    expect(await screen.findByRole("button", { name: "Play" })).toBeEnabled();
  });
});
