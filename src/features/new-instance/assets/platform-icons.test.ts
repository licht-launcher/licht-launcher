import path from "node:path";

import { build } from "vite";
import { expect, it } from "vitest";

it("bundles platform icons as local files allowed by the release CSP", async () => {
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
          "src/features/new-instance/assets/platform-icons.tsx",
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
  for (const icon of [
    "vanilla.svg",
    "fabric.png",
    "forge.png",
    "neoforge.png",
    "quilt.svg",
  ]) {
    expect(
      files.some(
        (file) =>
          file.type === "asset" &&
          file.originalFileNames.some((name) => path.basename(name) === icon),
      ),
      `${icon} must be emitted as a local asset`,
    ).toBe(true);
  }
  for (const file of files) {
    if (file.type === "chunk") {
      expect(file.code).not.toContain("data:image/");
    }
  }
});
