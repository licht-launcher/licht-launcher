import { type FrontendErrorSource, logFrontendError } from "@/api/commands";

/** Text written to the launcher log for an error, its stack and its component stack. */
export function describeError(error: unknown, componentStack?: string): string {
  const description = describeValue(error);
  return componentStack
    ? `${description}\nComponent stack:${componentStack}`
    : description;
}

function describeValue(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    const header = `${error.name}: ${error.message}`;
    if (!error.stack) {
      return header;
    }
    // V8 stacks start with the header; WebKit stacks list only the frames.
    return error.stack.startsWith(header)
      ? error.stack
      : `${header}\n${error.stack}`;
  }
  try {
    return JSON.stringify(error) ?? String(error);
  } catch {
    // Circular values cannot be serialized; their string form is still useful.
    return String(error);
  }
}

export function reportInterfaceError(
  source: FrontendErrorSource,
  error: unknown,
  componentStack?: string,
): void {
  logFrontendError(source, describeError(error, componentStack)).catch(
    (failure: unknown) => {
      console.error(
        "Could not write an interface error to the launcher log",
        failure,
        error,
      );
    },
  );
}

/** Reports uncaught errors and unhandled rejections; returns a function that stops it. */
export function installGlobalErrorReporting(target: EventTarget): () => void {
  const onError = (event: Event) => {
    const { error, message } = event as ErrorEvent;
    reportInterfaceError("window", error ?? message);
  };
  const onRejection = (event: Event) => {
    reportInterfaceError("promise", (event as PromiseRejectionEvent).reason);
  };
  target.addEventListener("error", onError);
  target.addEventListener("unhandledrejection", onRejection);
  return () => {
    target.removeEventListener("error", onError);
    target.removeEventListener("unhandledrejection", onRejection);
  };
}
