import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ModpackDescription } from "@/features/discover/components/modpack-description";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("ModpackDescription", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("renders the sanitized markup as elements", () => {
    render(
      <ModpackDescription html="<h2>Features</h2><p>Fast <strong>and</strong> pretty</p>" />,
    );

    expect(
      screen.getByRole("heading", { level: 2, name: "Features" }),
    ).toBeInTheDocument();
    expect(screen.getByText("and").tagName).toBe("STRONG");
  });

  it("opens links in the system browser instead of the webview", () => {
    render(
      <ModpackDescription html='<p><a href="https://download.fo/install">install <em>guide</em></a></p>' />,
    );

    const link = screen.getByRole("link", { name: "install guide" });
    const click = new MouseEvent("click", { bubbles: true, cancelable: true });
    fireEvent(link, click);

    expect(click.defaultPrevented).toBe(true);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("open_external_url", {
      url: "https://download.fo/install",
    });
  });

  it("opens a middle-clicked link in the system browser too", () => {
    render(
      <ModpackDescription html='<p><a href="https://download.fo/install">install</a></p>' />,
    );

    const link = screen.getByRole("link", { name: "install" });
    const middleClick = new MouseEvent("auxclick", {
      bubbles: true,
      cancelable: true,
      button: 1,
    });
    fireEvent(link, middleClick);

    expect(middleClick.defaultPrevented).toBe(true);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("open_external_url", {
      url: "https://download.fo/install",
    });
  });

  it("keeps the text of a link without an address", () => {
    render(<ModpackDescription html="<p><a>plain</a></p>" />);

    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByText("plain")).toBeInTheDocument();
  });
});
