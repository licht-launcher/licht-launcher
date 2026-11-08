import { fireEvent, render, screen } from "@testing-library/react";
import { toast } from "sonner";
import { describe, expect, it } from "vitest";

import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";

describe("Toaster", () => {
  it("shows a toast message", async () => {
    render(
      <>
        <Button type="button" onClick={() => toast("Instalação concluída")}>
          Avisar
        </Button>
        <Toaster />
      </>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Avisar" }));

    expect(await screen.findByText("Instalação concluída")).toBeInTheDocument();
  });
});
