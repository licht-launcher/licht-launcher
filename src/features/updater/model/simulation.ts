export const DEV_SIMULATE_UPDATE_EVENT = "licht-launcher:dev-simulate-update";
export const DEV_CLEAR_SIMULATED_UPDATE_EVENT =
  "licht-launcher:dev-clear-simulated-update";
// Shared with the settings window; the launcher clears it when it starts.
const key = "licht-launcher.dev-update-simulation";

export function getUpdateSimulation(): boolean {
  return import.meta.env.DEV && localStorage.getItem(key) === "true";
}

export function setUpdateSimulation(enabled: boolean) {
  if (!import.meta.env.DEV) return;
  if (enabled) localStorage.setItem(key, "true");
  else localStorage.removeItem(key);
  window.dispatchEvent(
    new Event(
      enabled ? DEV_SIMULATE_UPDATE_EVENT : DEV_CLEAR_SIMULATED_UPDATE_EVENT,
    ),
  );
}

export function clearUpdateSimulation() {
  localStorage.removeItem(key);
}

export function subscribeUpdateSimulation(listener: () => void) {
  const onStorage = (event: StorageEvent) => {
    if (event.key === key || event.key === null) listener();
  };
  window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, listener);
  window.addEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, listener);
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener(DEV_SIMULATE_UPDATE_EVENT, listener);
    window.removeEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, listener);
    window.removeEventListener("storage", onStorage);
  };
}
