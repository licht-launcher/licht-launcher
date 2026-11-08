import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

describe("Dialog", () => {
  it("opens when the trigger is activated", () => {
    render(
      <Dialog>
        <DialogTrigger>Abrir</DialogTrigger>
        <DialogContent>
          <DialogTitle>Título</DialogTitle>
          <DialogDescription>Descrição do diálogo.</DialogDescription>
        </DialogContent>
      </Dialog>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Abrir" }));

    expect(screen.getByRole("dialog", { name: "Título" })).toBeInTheDocument();
  });
});
