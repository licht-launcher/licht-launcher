import { browser } from "@wdio/globals";

/**
 * The launcher shows a "startup" window, then opens "main" and closes the startup
 * one. The WebDriver session begins on the startup window, so move to the window
 * that replaces it before any test runs.
 */
export async function switchToMainWindow(): Promise<void> {
  const startup = await currentWindow();
  let main: string | undefined;
  await browser.waitUntil(
    async () => {
      const handles = await browser.getWindowHandles();
      main = handles.find((handle) => handle !== startup);
      return main !== undefined;
    },
    {
      timeout: 60_000,
      timeoutMsg: "The main launcher window did not open after startup",
    },
  );
  if (main === undefined) {
    throw new Error("The main launcher window handle was not found");
  }
  await browser.switchToWindow(main);
}

async function currentWindow(): Promise<string | undefined> {
  try {
    return await browser.getWindowHandle();
  } catch (error: unknown) {
    // Startup may already have closed its window; then every handle left is "main".
    if (error instanceof Error && error.message.includes("no such window")) {
      return undefined;
    }
    throw error;
  }
}
