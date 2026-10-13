import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";

import { Toaster } from "@/components/ui/sonner";
import { invoke } from "@/features/settings/fixtures/invoke";
import { snapshot } from "@/features/settings/fixtures/snapshot";
import { SettingsDialog } from "@/features/settings/settings-dialog";

export { invoke } from "@/features/settings/fixtures/invoke";
export { snapshot } from "@/features/settings/fixtures/snapshot";

export function mockSettingsCommands(implement?: (command: string) => unknown) {
  invoke.mockReset();
  invoke.mockImplementation((command: string) => {
    const custom = implement?.(command);
    if (custom !== undefined) {
      return custom;
    }
    if (command === "get_settings") {
      return Promise.resolve(snapshot);
    }
    if (command === "list_microsoft_accounts") {
      return Promise.resolve([]);
    }
    if (command === "active_microsoft_account") {
      return Promise.resolve(null);
    }
    if (command === "runtime_java") {
      return Promise.resolve({
        java25: null,
        java21: null,
        java17: null,
        java8: null,
      });
    }
    if (command === "java_installation_status") {
      return Promise.resolve({
        java25: false,
        java21: false,
        java17: false,
        java8: false,
      });
    }
    if (command === "detect_java_installations") {
      return Promise.resolve({
        java25: [],
        java21: [],
        java17: [],
        java8: [],
      });
    }
    if (command === "save_settings") {
      return Promise.resolve(snapshot.settings);
    }
    if (command === "custom_title_bar_active") {
      return Promise.resolve(true);
    }
    if (command === "restart_app") {
      return Promise.resolve();
    }
    return Promise.resolve(null);
  });
}

export function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SettingsDialog open onOpenChange={() => {}} />
      <Toaster />
    </QueryClientProvider>,
  );
}
