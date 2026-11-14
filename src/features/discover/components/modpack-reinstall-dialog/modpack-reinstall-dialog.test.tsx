import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ModpackReinstallDialog } from "./modpack-reinstall-dialog";

function renderDialog({
  open = true,
  pending = false,
  onOpenChange = vi.fn(),
  onConfirm = vi.fn(),
} = {}) {
  render(
    <ModpackReinstallDialog
      open={open}
      pending={pending}
      onOpenChange={onOpenChange}
      onConfirm={onConfirm}
    />,
  );
  return { onOpenChange, onConfirm };
}

describe("ModpackReinstallDialog", () => {
  it("explains that the pack is already installed", () => {
    renderDialog();

    const dialog = screen.getByRole("dialog", {
      name: "Install this modpack again?",
    });
    expect(dialog).toHaveTextContent(
      "An instance of this modpack already exists. Are you sure you want to install it?",
    );
  });

  it("installs when the user confirms", () => {
    const { onConfirm } = renderDialog();

    fireEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Install",
      }),
    );

    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("closes without installing when the user cancels", () => {
    const { onConfirm, onOpenChange } = renderDialog();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("does not confirm while an install is already running", () => {
    const { onConfirm } = renderDialog({ pending: true });

    const install = within(screen.getByRole("dialog")).getByRole("button", {
      name: "Install",
    });
    expect(install).toBeDisabled();
    fireEvent.click(install);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("stays closed until an existing pack is found", () => {
    renderDialog({ open: false });

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
