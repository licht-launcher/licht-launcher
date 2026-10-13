import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlaySection } from "@/features/instances/sections/play-section";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const instance: InstanceEntry = {
  folder: "classic",
  name: "Classic",
  versionId: "1.5.2",
  minMemoryMb: 512,
  maxMemoryMb: 2048,
  jvmArguments: [],
  fullscreen: false,
  width: null,
  height: null,
  overrideWindow: false,
  overrideMemory: false,
  overrideJava: false,
  overrideJvmArguments: false,
  javaPath: null,
  lastOpenedAt: new Date(Date.now() - 5 * 60_000).toISOString(),
  iconPath: null,
};

describe("PlaySection", () => {
  it("shows tooltips for version, loader, and last played", async () => {
    render(
      <PlaySection
        instance={instance}
        busy={false}
        playDisabled={false}
        phase="idle"
        installing={false}
        progress={null}
        onPlay={vi.fn()}
        onStop={vi.fn()}
        onConfigure={vi.fn()}
      />,
    );

    fireEvent.focus(screen.getByRole("button", { name: /1\.5\.2/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Game version",
    );

    fireEvent.blur(screen.getByRole("button", { name: /1\.5\.2/ }));
    fireEvent.focus(screen.getByRole("button", { name: /Vanilla/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Loader");

    fireEvent.blur(screen.getByRole("button", { name: /Vanilla/ }));
    fireEvent.focus(screen.getByRole("button", { name: /minutes ago/ }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Last played");
  });

  it("places install progress below the title row", () => {
    const { container } = render(
      <PlaySection
        instance={instance}
        busy={false}
        playDisabled={false}
        phase="idle"
        installing
        progress={{
          finished: 1,
          failed: 0,
          total: 10,
          bytesDone: 100,
          bytesTotal: 1000,
        }}
        onPlay={vi.fn()}
        onStop={vi.fn()}
        onConfigure={vi.fn()}
      />,
    );

    const root = container.firstElementChild;
    expect(root).toHaveClass("flex-col");
    expect(screen.getByRole("progressbar", { name: "Install progress" })).toBe(
      root?.lastElementChild?.querySelector('[role="progressbar"]'),
    );
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Configure Classic" }),
    ).toBeInTheDocument();
  });
});
