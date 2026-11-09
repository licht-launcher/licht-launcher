import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Button } from "@/components/ui/button";
import { CreateInstanceMenu } from "@/features/new-instance/components/create-instance-menu";

const upload = vi.hoisted(() => ({ mutate: vi.fn(), isPending: false }));

vi.mock("@/features/new-instance/hooks/use-upload-instance", () => ({
  useUploadInstance: () => ({ upload }),
}));

beforeEach(() => {
  upload.mutate.mockReset();
  upload.isPending = false;
});

function renderMenu() {
  const onScratch = vi.fn();
  render(
    <CreateInstanceMenu onScratch={onScratch}>
      <Button type="button">Create instance</Button>
    </CreateInstanceMenu>,
  );
  const trigger = screen.getByRole("button", { name: "Create instance" });
  return { trigger, onScratch };
}

async function open(trigger: HTMLElement) {
  fireEvent.pointerDown(trigger, {
    button: 0,
    ctrlKey: false,
    pointerType: "mouse",
  });
  return screen.findByRole("menu", { name: "Create instance" });
}

describe("CreateInstanceMenu", () => {
  it("lists scratch, modpack and upload", async () => {
    const { trigger } = renderMenu();

    await open(trigger);

    const names = screen
      .getAllByRole("menuitem")
      .map((item) => item.textContent);
    expect(names).toEqual([
      "From scratch",
      "From a modpack",
      "Upload a pack file",
    ]);
  });

  it("keeps the modpack option disabled until Discover exists", async () => {
    const { trigger, onScratch } = renderMenu();
    await open(trigger);

    const modpack = screen.getByRole("menuitem", { name: "From a modpack" });
    expect(modpack).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(modpack);
    expect(onScratch).not.toHaveBeenCalled();
    expect(upload.mutate).not.toHaveBeenCalled();
  });

  it("opens the scratch form from the first option", async () => {
    const { trigger, onScratch } = renderMenu();
    await open(trigger);

    fireEvent.click(screen.getByRole("menuitem", { name: "From scratch" }));

    expect(onScratch).toHaveBeenCalledOnce();
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(upload.mutate).not.toHaveBeenCalled();
  });

  it("starts the upload straight from the last option", async () => {
    const { trigger, onScratch } = renderMenu();
    await open(trigger);

    fireEvent.click(
      screen.getByRole("menuitem", { name: "Upload a pack file" }),
    );

    expect(upload.mutate).toHaveBeenCalledOnce();
    expect(onScratch).not.toHaveBeenCalled();
  });

  it("does not start a second upload while one is running", async () => {
    upload.isPending = true;
    const { trigger } = renderMenu();
    await open(trigger);

    const item = screen.getByRole("menuitem", { name: "Upload a pack file" });
    expect(item).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(item);
    expect(upload.mutate).not.toHaveBeenCalled();
  });
});
