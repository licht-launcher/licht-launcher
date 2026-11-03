import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { LoaderOptions } from "@/features/new-instance/components/loader-options";
import {
  initialScratchState,
  type ScratchState,
} from "@/features/new-instance/model/create-state";

function renderOptions(
  state: Partial<ScratchState> = {},
  props: Partial<Parameters<typeof LoaderOptions>[0]> = {},
) {
  const onChange = vi.fn();
  render(
    <LoaderOptions
      state={{
        ...initialScratchState,
        platform: "quilt",
        gameVersion: "26.3",
        ...state,
      }}
      loaderState="ready"
      stableMissing={false}
      loaderVersions={["0.30.2", "0.30.1"]}
      onChange={onChange}
      {...props}
    />,
  );
  return { onChange };
}

describe("LoaderOptions", () => {
  it("offers the channels as cards: Stable, Latest and Other", () => {
    renderOptions();

    const group = screen.getByRole("group", { name: "Loader channel" });
    const names = [...group.querySelectorAll("button")].map(
      (button) => button.textContent,
    );
    expect(names).toEqual(["Stable", "Latest", "Other"]);
    expect(screen.getByRole("button", { name: "Stable" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Latest" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("flows the channel cards at the width of their name", () => {
    renderOptions();

    const group = screen.getByRole("group", { name: "Loader channel" });
    expect(group).toHaveClass("flex", "flex-wrap");
    expect(screen.getByRole("button", { name: "Other" })).toHaveClass("w-auto");
  });

  it("reports the channel that was picked", () => {
    const { onChange } = renderOptions();

    fireEvent.click(screen.getByRole("button", { name: "Latest" }));
    expect(onChange).toHaveBeenCalledWith({
      loaderChannel: "latest",
      loaderVersion: "",
    });
    fireEvent.click(screen.getByRole("button", { name: "Other" }));
    expect(onChange).toHaveBeenCalledWith({
      loaderChannel: "manual",
      loaderVersion: "",
    });
  });

  it("disables Stable and says why when the loader has no stable build", async () => {
    const { onChange } = renderOptions(
      { loaderChannel: "latest" },
      { stableMissing: true },
    );

    const stable = screen.getByRole("button", { name: "Stable" });
    expect(stable).toBeDisabled();
    expect(screen.getByRole("button", { name: "Latest" })).toBeEnabled();
    fireEvent.focus(stable.parentElement as HTMLElement);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "No stable Quilt build for 26.3.",
    );
    fireEvent.click(stable);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("disables every channel when the loader has no build at all", async () => {
    renderOptions(
      { loaderChannel: "latest" },
      { stableMissing: true, loaderState: "empty" },
    );

    for (const name of ["Stable", "Latest", "Other"]) {
      expect(screen.getByRole("button", { name })).toBeDisabled();
    }
    fireEvent.focus(
      screen.getByRole("button", { name: "Latest" })
        .parentElement as HTMLElement,
    );
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Quilt has no build for 26.3.",
    );
  });

  it("prints no status text under the cards", () => {
    for (const loaderState of ["checking", "empty", "ready"] as const) {
      const { unmount } = render(
        <LoaderOptions
          state={{
            ...initialScratchState,
            platform: "quilt",
            gameVersion: "26.3",
            loaderChannel: "latest",
          }}
          loaderState={loaderState}
          stableMissing
          loaderVersions={[]}
          onChange={() => {}}
        />,
      );
      expect(screen.queryByText(/Checking .* builds/)).toBeNull();
      expect(screen.queryByText(/has no build/)).toBeNull();
      expect(screen.queryByText(/No stable/)).toBeNull();
      unmount();
    }
  });

  it("disables the channels with a reason when the check failed, without a retry button", async () => {
    renderOptions({}, { loaderState: "error" });

    for (const name of ["Stable", "Latest", "Other"]) {
      expect(screen.getByRole("button", { name })).toBeDisabled();
    }
    expect(screen.queryByText(/Could not check/)).toBeNull();
    fireEvent.focus(
      screen.getByRole("button", { name: "Stable" })
        .parentElement as HTMLElement,
    );
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Could not check Quilt builds for 26.3.",
    );
    expect(screen.queryByRole("button", { name: /retry/i })).toBeNull();
  });

  it("keeps every channel available while the builds are still loading", () => {
    renderOptions({}, { loaderState: "checking" });

    for (const name of ["Stable", "Latest", "Other"]) {
      expect(screen.getByRole("button", { name })).toBeEnabled();
    }
  });

  it("asks for the loader version only under Other", async () => {
    const { rerender } = render(
      <LoaderOptions
        state={{
          ...initialScratchState,
          platform: "quilt",
          gameVersion: "26.3",
        }}
        loaderState="ready"
        stableMissing={false}
        loaderVersions={["0.30.2", "0.30.1"]}
        onChange={() => {}}
      />,
    );
    expect(
      screen.queryByRole("combobox", { name: "Loader version" }),
    ).toBeNull();

    rerender(
      <LoaderOptions
        state={{
          ...initialScratchState,
          platform: "quilt",
          gameVersion: "26.3",
          loaderChannel: "manual",
          loaderVersion: "0.30.2",
        }}
        loaderState="ready"
        stableMissing={false}
        loaderVersions={["0.30.2", "0.30.1"]}
        onChange={() => {}}
      />,
    );
    expect(
      await screen.findByRole("combobox", { name: "Loader version" }),
    ).toBeVisible();
    expect(
      document
        .querySelector("[data-slot='reveal']")
        ?.contains(screen.getByRole("combobox", { name: "Loader version" })),
    ).toBe(true);
  });
});
