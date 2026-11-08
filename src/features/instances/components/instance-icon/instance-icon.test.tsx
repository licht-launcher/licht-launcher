import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import grassFallback from "@/assets/instance-grass.svg";
import { InstanceIcon } from "@/features/instances/components/instance-icon";

const convertFileSrcMock = vi.fn((path: string) => `asset://localhost/${path}`);

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => convertFileSrcMock(path),
}));

describe("InstanceIcon", () => {
  it("falls back to grass when convertFileSrc is unavailable", () => {
    convertFileSrcMock.mockImplementationOnce(() => {
      throw new Error(
        "Cannot read properties of undefined (reading 'metadata')",
      );
    });
    const { container } = render(
      <InstanceIcon iconPath="C:/data/instances/classic/icon.png" />,
    );
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      grassFallback,
    );
  });

  it("renders the grass fallback when there is no icon path", () => {
    const { container } = render(<InstanceIcon />);
    const tile = container.firstElementChild;
    expect(tile).toHaveAttribute("aria-hidden", "true");
    const img = container.querySelector("img");
    expect(img).toHaveAttribute("src", grassFallback);
  });

  it("uses convertFileSrc when an icon path is provided", () => {
    const { container } = render(
      <InstanceIcon iconPath="C:/data/instances/classic/icon.png" />,
    );
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "asset://localhost/C:/data/instances/classic/icon.png",
    );
  });
});
