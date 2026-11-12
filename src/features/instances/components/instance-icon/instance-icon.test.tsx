import path from "node:path";

import { render } from "@testing-library/react";
import { build } from "vite";
import { describe, expect, it, vi } from "vitest";

import grassFallback from "@/assets/instance-grass.svg?no-inline";
import { InstanceIcon } from "@/features/instances/components/instance-icon";

const convertFileSrcMock = vi.fn((path: string) => `asset://localhost/${path}`);

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => convertFileSrcMock(path),
}));

describe("InstanceIcon", () => {
  it("falls back to grass when convertFileSrc is unavailable", () => {
    convertFileSrcMock.mockImplementationOnce(() => {
      throw new Error(
        "Cannot read properties of undefined (reading 'metadata')",
      );
    });
    const { container } = render(
      <InstanceIcon iconPath="C:/data/instances/classic/icon.png" />,
    );
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      grassFallback,
    );
  });

  it("renders the grass fallback when there is no icon path", () => {
    const { container } = render(<InstanceIcon />);
    const tile = container.firstElementChild;
    expect(tile).toHaveAttribute("aria-hidden", "true");
    const img = container.querySelector("img");
    expect(img).toHaveAttribute("src", grassFallback);
  });

  it("uses convertFileSrc when an icon path is provided", () => {
    const { container } = render(
      <InstanceIcon iconPath="C:/data/instances/classic/icon.png" />,
    );
    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "asset://localhost/C:/data/instances/classic/icon.png",
    );
  });
});

it("bundles the grass fallback as a local file allowed by the release CSP", async () => {
  const root = process.cwd();
  const result = await build({
    configFile: false,
    root,
    logLevel: "silent",
    resolve: {
      alias: { "@": path.resolve(root, "src") },
    },
    build: {
      write: false,
      minify: false,
      rolldownOptions: {
        input: path.resolve(
          root,
          "src/features/instances/components/instance-icon/instance-icon.tsx",
        ),
        preserveEntrySignatures: "strict",
      },
    },
  });

  const outputs = Array.isArray(result) ? result : [result];
  const files = outputs.flatMap((output) => {
    if (!("output" in output)) {
      throw new Error("Expected a completed build, not a watcher");
    }
    return output.output;
  });
  expect(
    files.some(
      (file) =>
        file.type === "asset" &&
        file.originalFileNames.some(
          (name) => path.basename(name) === "instance-grass.svg",
        ),
    ),
    "instance-grass.svg must be emitted as a local asset",
  ).toBe(true);
  for (const file of files) {
    if (file.type === "chunk") {
      expect(file.code).not.toContain("data:image/");
    }
  }
});
