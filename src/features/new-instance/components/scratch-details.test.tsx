import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ScratchDetails } from "@/features/new-instance/components/scratch-details";
import {
  initialScratchState,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

const versions: VersionSummary[] = [
  { id: "1.20.1", versionType: "release", installed: false },
  { id: "1.19.4", versionType: "release", installed: false },
  { id: "24w14a", versionType: "snapshot", installed: false },
];

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    function (this: HTMLElement) {
      return this.dataset.slot === "combobox-viewport" ? 256 : 0;
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(320);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function renderDetails(
  state: Partial<ScratchState> = {},
  props: Partial<Parameters<typeof ScratchDetails>[0]> = {},
) {
  const onChange = vi.fn();
  render(
    <ScratchDetails
      state={{ ...initialScratchState, gameVersion: "1.20.1", ...state }}
      takenNames={[]}
      versions={versions}
      versionsPending={false}
      versionsError={false}
      loaderVersions={[]}
      loaderVersionsPending={false}
      onRetryVersions={() => {}}
      onChange={onChange}
      {...props}
    />,
  );
  return { onChange };
}

describe("ScratchDetails", () => {
  it("stacks name, platform and game version one below the other", () => {
    renderDetails();

    const headings = screen
      .getAllByRole("heading", { level: 3 })
      .map((heading) => heading.textContent);
    expect(headings).toEqual([
      "Name your instance",
      "Choose a platform",
      "Game version",
    ]);
  });

  it("puts the loader after the game version for platforms with a loader", () => {
    renderDetails({ platform: "fabric" });

    const headings = screen
      .getAllByRole("heading", { level: 3 })
      .map((heading) => heading.textContent);
    expect(headings).toEqual([
      "Name your instance",
      "Choose a platform",
      "Game version",
      "Loader version",
    ]);
  });

  it("goes back to the stable channel whenever the platform changes", () => {
    const { onChange } = renderDetails({
      platform: "fabric",
      loaderChannel: "manual",
      loaderVersion: "0.15.11",
    });

    fireEvent.click(screen.getByRole("button", { name: /^Forge/ }));
    expect(onChange).toHaveBeenCalledWith({
      platform: "forge",
      loaderChannel: "stable",
      loaderVersion: "",
    });
  });

  it("suggests the default name as a placeholder", () => {
    renderDetails({ platform: "fabric", gameVersion: "26.3" });

    expect(screen.getByLabelText("Instance Name")).toHaveAttribute(
      "placeholder",
      "Fabric 26.3",
    );
  });

  it("tints the snapshots switch yellow", () => {
    renderDetails();

    expect(
      screen.getByRole("switch", { name: "Include snapshots" }),
    ).toHaveClass("data-[state=checked]:bg-warning");
  });

  it("selects the game version from a typeable select", () => {
    const { onChange } = renderDetails();

    const field = screen.getByRole("combobox", { name: "Game version" });
    expect(field).toHaveValue("1.20.1");
    fireEvent.focus(field);
    fireEvent.change(field, { target: { value: "1.19" } });
    fireEvent.click(screen.getByRole("option", { name: "1.19.4" }));
    expect(onChange).toHaveBeenCalledWith({
      gameVersion: "1.19.4",
      loaderVersion: "",
    });
  });

  it("offers snapshots only when asked", () => {
    const { onChange } = renderDetails();

    const field = screen.getByRole("combobox", { name: "Game version" });
    fireEvent.focus(field);
    expect(screen.queryByRole("option", { name: "24w14a" })).toBeNull();
    fireEvent.click(screen.getByRole("switch", { name: "Include snapshots" }));
    expect(onChange).toHaveBeenCalledWith({ includeSnapshots: true });
  });

  it("drops a hidden snapshot so the newest release is picked again", () => {
    const { onChange } = renderDetails({
      gameVersion: "24w14a",
      includeSnapshots: true,
    });

    fireEvent.click(screen.getByRole("switch", { name: "Include snapshots" }));
    expect(onChange).toHaveBeenCalledWith({
      includeSnapshots: false,
      gameVersion: "",
      loaderVersion: "",
    });
  });

  it("waits for the version list and lets the user retry after an error", () => {
    const onRetryVersions = vi.fn();
    const { onChange } = renderDetails(
      { gameVersion: "" },
      { versionsPending: true },
    );
    expect(
      screen.getByRole("combobox", { name: "Game version" }),
    ).toBeDisabled();
    expect(onChange).not.toHaveBeenCalled();

    render(
      <ScratchDetails
        state={initialScratchState}
        takenNames={[]}
        versions={[]}
        versionsPending={false}
        versionsError
        loaderVersions={[]}
        loaderVersionsPending={false}
        onRetryVersions={onRetryVersions}
        onChange={() => {}}
      />,
    );
    expect(screen.getByText(/Could not load the version list/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(onRetryVersions).toHaveBeenCalledOnce();
  });
});
