import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createHashHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Toaster } from "@/components/ui/sonner";
import { StartupScreen } from "@/features/startup/startup-screen";

import { routeTree } from "./routeTree.gen";
import "./styles/tokens.css";

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

if (root === null) {
  throw new Error("The application root element was not found.");
}

const startupWindow =
  isTauri() && getCurrentWindow().label.startsWith("startup");

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {startupWindow ? (
        <StartupScreen />
      ) : (
        <>
          <RouterProvider router={router} />
          <Toaster />
        </>
      )}
    </QueryClientProvider>
  </StrictMode>,
);
