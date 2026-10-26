import { render, screen } from "@testing-library/react";
import { Download } from "lucide-react";
import { describe, expect, it } from "vitest";
import { EmptyState } from "@/components/ui/empty-state";

describe("EmptyState", () => {
  it("announces its title and explains the empty state", () => {
    render(
      <EmptyState
        icon={<Download />}
        title="No downloads in progress"
        description="Your next download will appear here."
      />,
    );
    expect(
      screen.getByRole("status", { name: "No downloads in progress" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "No downloads in progress" }),
    ).toBeVisible();
    expect(
      screen.getByText("Your next download will appear here."),
    ).toBeVisible();
  });
});
