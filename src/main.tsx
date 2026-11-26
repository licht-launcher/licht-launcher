import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createHashHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot, type RootOptions } from "react-dom/client";

import { Toaster } from "@/components/ui/sonner";
import { clearStorageLoadingPreview } from "@/features/downloads/hooks/use-storage-loading-preview";
import { useLauncherTheme } from "@/features/settings/hooks/use-launcher-theme";
import { SettingsWindow } from "@/features/settings/settings-window";
import { StartupGate } from "@/features/startup/startup-gate";
import { StartupScreen } from "@/features/startup/startup-screen";
import { clearUpdateSimulation } from "@/features/updater/model/simulation";
import {
  installGlobalErrorReporting,
  reportInterfaceError,
} from "@/lib/error-reporting";

import { routeTree } from "./routeTree.gen";
import "./styles/tokens.css";

// Only end-to-end test builds (`vite build --mode e2e`) load the WebdriverIO bridge;
// Vite replaces MODE at build time, so other builds drop this import.
if (import.meta.env.MODE === "e2e") {
  await import("@wdio/tauri-plugin");
}

const queryClient = new QueryClient();

const router = createRouter({
  routeTree,
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

const root = document.getElementById("root");

// The native app writes interface errors to the launcher log; the web preview keeps
// React's default console reporting.
const rootOptions: RootOptions = {};
if (isTauri()) {
  installGlobalErrorReporting(window);
  const reportRenderError = (
    error: unknown,
    info: { componentStack?: string },
  ) => {
    reportInterfaceError("render", error, info.componentStack);
  };
  rootOptions.onCaughtError = reportRenderError;
  rootOptions.onUncaughtError = reportRenderError;
}

if (root === null) {
  throw new Error("The application root element was not found.");
}

// Developer startup previews open in their own window; the launcher window
// shows startup itself and then the launcher.
const windowLabel = isTauri() ? getCurrentWindow().label : "main";
const previewWindow = windowLabel.startsWith("startup-preview-");

if (windowLabel === "main") {
  clearUpdateSimulation();
  clearStorageLoadingPreview();
}

function Startup() {
  useLauncherTheme(true);
  return <StartupScreen />;
}

function Launcher() {
  const theme = useLauncherTheme(false);
  return (
    <>
      <RouterProvider router={router} />
      <Toaster theme={theme} />
    </>
  );
}

function Settings() {
  const theme = useLauncherTheme(false);
  return (
    <>
      <SettingsWindow />
      <Toaster theme={theme} />
    </>
  );
}

function App() {
  if (windowLabel === "settings") {
    return <Settings />;
  }
  return previewWindow ? (
    <Startup />
  ) : (
    <StartupGate fallback={<Startup />}>
      <Launcher />
    </StartupGate>
  );
}

createRoot(root, rootOptions).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
