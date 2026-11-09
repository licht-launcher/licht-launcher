import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

import { browser } from "@wdio/globals";

/**
 * Saves a screenshot and the page HTML of the window under test, so a failure on CI
 * can be inspected from the uploaded logs instead of being reproduced locally.
 */
export async function captureFailure(
  directory: string,
  testTitle: string,
): Promise<void> {
  mkdirSync(directory, { recursive: true });
  const baseName = path.join(directory, toFileName(testTitle));
  await browser.saveScreenshot(`${baseName}.png`);
  writeFileSync(`${baseName}.html`, await browser.getPageSource(), "utf8");
}

function toFileName(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}
