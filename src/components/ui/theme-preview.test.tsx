import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChoiceCard } from "@/components/ui/choice-card";
import { ThemePreview } from "@/components/ui/theme-preview";
import type { LauncherTheme } from "@/lib/generated/LauncherTheme";

describe("ThemePreview", () => {
  it.each<LauncherTheme>(["light", "dark", "system"])(
    "renders a named window illustration for %s",
    (theme) => {
      render(<ThemePreview theme={theme} />);
      expect(
        screen.getByRole("img", {
          name: new RegExp(`${theme} theme preview`, "i"),
        }),
      ).toBeVisible();
    },
  );

  it("keeps a theme card's accessible name concise when the preview is decorative", () => {
    render(
      <ChoiceCard title="Light" selected onSelect={() => {}}>
        <span aria-hidden="true">
          <ThemePreview theme="light" />
        </span>
      </ChoiceCard>,
    );
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });
});
