import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { InstanceIcon } from "@/features/instances/components/instance-icon";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));

describe("InstanceIcon", () => {
  it("renders the grass fallback when there is no icon path", () => {
    const { container } = render(<InstanceIcon />);
    const tile = container.firstElementChild;
    expect(tile).toHaveAttribute("aria-hidden", "true");
    expect(tile).toHaveClass("size-6", "rounded-md");
    const img = container.querySelector("img");
    expect(img).toHaveAttribute("src");
    expect(img?.getAttribute("src")).not.toContain("asset://");
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

  it("supports a larger tile for the detail header", () => {
    const { container } = render(<InstanceIcon size="md" />);
    expect(container.firstElementChild).toHaveClass("size-10");
  });
});
