import { fireEvent, render, screen } from "@testing-library/react";
import { UserRound } from "lucide-react";
import { describe, expect, it } from "vitest";

import { Input } from "@/components/ui/input";

describe("Input", () => {
  it("accepts typed text", () => {
    render(<Input aria-label="Nome" />);

    fireEvent.change(screen.getByRole("textbox", { name: "Nome" }), {
      target: { value: "Lilie" },
    });

    expect(screen.getByRole("textbox", { name: "Nome" })).toHaveValue("Lilie");
  });

  it("renders a start icon without blocking typing", () => {
    render(
      <Input
        aria-label="Offline username"
        startIcon={<UserRound data-testid="start-icon" aria-hidden />}
      />,
    );

    expect(screen.getByTestId("start-icon")).toBeInTheDocument();
    fireEvent.change(
      screen.getByRole("textbox", { name: "Offline username" }),
      {
        target: { value: "Steve" },
      },
    );
    expect(
      screen.getByRole("textbox", { name: "Offline username" }),
    ).toHaveValue("Steve");
  });
});
