import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";

describe("Button", () => {
  it("runs the click handler", () => {
    const onClick = vi.fn();

    render(<Button onClick={onClick}>Salvar</Button>);
    fireEvent.click(screen.getByRole("button", { name: "Salvar" }));

    expect(onClick).toHaveBeenCalledOnce();
  });
});
