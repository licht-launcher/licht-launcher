import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { InstanceIcon } from "@/features/instances/components/instance-icon";

describe("InstanceIcon", () => {
  it("renders a decorative placeholder tile", () => {
    const { container } = render(<InstanceIcon />);
    const tile = container.firstElementChild;
    expect(tile).toHaveAttribute("aria-hidden", "true");
    expect(tile).toHaveClass("size-6", "rounded-md");
  });

  it("supports a larger tile for the detail header", () => {
    const { container } = render(<InstanceIcon size="md" />);
    expect(container.firstElementChild).toHaveClass("size-10");
  });
});
