import { fireEvent, render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SkinHead } from "@/components/ui/skin-head";
import { playerHeadPathFixture } from "@/features/settings/fixtures/snapshot";

const { convertFileSrc } = vi.hoisted(() => ({ convertFileSrc: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc }));

describe("SkinHead", () => {
  beforeEach(() => {
    convertFileSrc
      .mockReset()
      .mockImplementation((path: string) => `asset://localhost/${path}`);
  });

  it("shows a neutral placeholder while the local head is unavailable", () => {
    const { container } = render(<SkinHead />);
    expect(container.querySelector("svg")).toBeVisible();
    expect(container.querySelector("img")).toBeNull();
  });

  it("renders the cached head through Tauri's local asset protocol", () => {
    const { container } = render(<SkinHead headPath={playerHeadPathFixture} />);
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      `asset://localhost/${playerHeadPathFixture}`,
    );
    expect(container.querySelector("img")).toHaveAttribute(
      "draggable",
      "false",
    );
    expect(container.querySelector("svg")).toBeNull();
  });

  it("handles an unreadable image and renders another account's cached head", () => {
    const { container, rerender } = render(
      <SkinHead headPath={playerHeadPathFixture} />,
    );
    const image = container.querySelector("img");
    if (!image) throw new Error("Missing cached head");
    fireEvent.error(image);
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("svg")).toBeVisible();
    const other = "C:\\Licht\\cache\\player-heads\\other.png";
    rerender(<SkinHead headPath={other} />);
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      `asset://localhost/${other}`,
    );
  });

  it("keeps the placeholder if the native asset protocol is unavailable", () => {
    convertFileSrc.mockImplementation(() => {
      throw new Error("Native bridge unavailable");
    });
    const { container } = render(<SkinHead headPath={playerHeadPathFixture} />);
    expect(container.querySelector("svg")).toBeVisible();
    expect(container.querySelector("img")).toBeNull();
  });
});
