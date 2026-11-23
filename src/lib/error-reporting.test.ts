import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { logFrontendError } from "@/api/commands";
import {
  describeError,
  installGlobalErrorReporting,
  reportInterfaceError,
} from "@/lib/error-reporting";

vi.mock("@/api/commands", () => ({
  logFrontendError: vi.fn(),
}));

describe("describeError", () => {
  it("uses a stack that already starts with the error name and message", () => {
    const error = new TypeError("x is undefined");
    error.stack = "TypeError: x is undefined\n    at Panel (panel.tsx:3:7)";

    expect(describeError(error)).toBe(
      "TypeError: x is undefined\n    at Panel (panel.tsx:3:7)",
    );
  });

  it("adds the name and message when the stack lacks them", () => {
    const error = new TypeError("x is undefined");
    error.stack = "Panel@panel.tsx:3:7";

    expect(describeError(error)).toBe(
      "TypeError: x is undefined\nPanel@panel.tsx:3:7",
    );
  });

  it("describes an error without a stack by its name and message", () => {
    const error = new RangeError("out of range");
    error.stack = undefined;

    expect(describeError(error)).toBe("RangeError: out of range");
  });

  it("keeps string reasons as they are", () => {
    expect(describeError("Instance is still preparing")).toBe(
      "Instance is still preparing",
    );
  });

  it("serializes other values and falls back for circular ones", () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;

    expect(describeError({ code: 7 })).toBe('{"code":7}');
    expect(describeError(undefined)).toBe("undefined");
    expect(describeError(circular)).toBe("[object Object]");
  });

  it("appends the React component stack", () => {
    expect(describeError("boom", "\n    at Panel\n    at App")).toBe(
      "boom\nComponent stack:\n    at Panel\n    at App",
    );
  });
});

describe("reportInterfaceError", () => {
  beforeEach(() => {
    vi.mocked(logFrontendError).mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("sends the described error with its source to the launcher log", () => {
    reportInterfaceError("render", "boom", "\n    at Panel");

    expect(logFrontendError).toHaveBeenCalledExactlyOnceWith(
      "render",
      "boom\nComponent stack:\n    at Panel",
    );
  });

  it("falls back to the console when the launcher log cannot be reached", async () => {
    const failure = new Error("IPC unavailable");
    vi.mocked(logFrontendError).mockRejectedValueOnce(failure);
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    reportInterfaceError("promise", "boom");

    await vi.waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith(
        "Could not write an interface error to the launcher log",
        failure,
        "boom",
      );
    });
  });
});

describe("installGlobalErrorReporting", () => {
  beforeEach(() => {
    vi.mocked(logFrontendError).mockReset().mockResolvedValue(undefined);
  });

  it("reports uncaught errors and unhandled rejections until it is removed", () => {
    const target = new EventTarget();
    const uninstall = installGlobalErrorReporting(target);
    const error = new Error("uncaught");
    error.stack = undefined;

    target.dispatchEvent(
      new ErrorEvent("error", { error, message: "ignored" }),
    );
    const rejection = Object.assign(new Event("unhandledrejection"), {
      reason: "rejected",
    });
    target.dispatchEvent(rejection);

    expect(logFrontendError).toHaveBeenNthCalledWith(
      1,
      "window",
      "Error: uncaught",
    );
    expect(logFrontendError).toHaveBeenNthCalledWith(2, "promise", "rejected");

    uninstall();
    target.dispatchEvent(new ErrorEvent("error", { message: "after removal" }));
    expect(logFrontendError).toHaveBeenCalledTimes(2);
  });

  it("uses the event message when an error event carries no error", () => {
    const target = new EventTarget();
    const uninstall = installGlobalErrorReporting(target);

    target.dispatchEvent(new ErrorEvent("error", { message: "Script error." }));

    expect(logFrontendError).toHaveBeenCalledExactlyOnceWith(
      "window",
      "Script error.",
    );
    uninstall();
  });
});
