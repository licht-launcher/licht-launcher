import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

describe("Popover", () => {
  it("opens content from the trigger", async () => {
    render(
      <Popover>
        <PopoverTrigger asChild>
          <Button type="button">Open</Button>
        </PopoverTrigger>
        <PopoverContent>Panel</PopoverContent>
      </Popover>,
    );

    expect(screen.queryByText("Panel")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(await screen.findByText("Panel")).toBeInTheDocument();
  });
});
