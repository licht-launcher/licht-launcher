import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

describe("Tooltip", () => {
  it("shows help text on focus", async () => {
    render(
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger>Ajuda</TooltipTrigger>
          <TooltipContent>Texto da dica</TooltipContent>
        </Tooltip>
      </TooltipProvider>,
    );

    fireEvent.focus(screen.getByRole("button", { name: "Ajuda" }));

    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Texto da dica",
    );
  });
});
