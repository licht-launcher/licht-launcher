export const DEV_SIMULATE_UPDATE_EVENT = "licht-launcher:dev-simulate-update";
export const DEV_CLEAR_SIMULATED_UPDATE_EVENT =
  "licht-launcher:dev-clear-simulated-update";
const key = "licht-launcher.dev-update-simulation";

export function getUpdateSimulation(): boolean {
  return import.meta.env.DEV && sessionStorage.getItem(key) === "true";
}

export function setUpdateSimulation(enabled: boolean) {
  if (!import.meta.env.DEV) return;
  if (enabled) sessionStorage.setItem(key, "true");
  else sessionStorage.removeItem(key);
  window.dispatchEvent(
    new Event(
      enabled ? DEV_SIMULATE_UPDATE_EVENT : DEV_CLEAR_SIMULATED_UPDATE_EVENT,
    ),
  );
}

export function subscribeUpdateSimulation(listener: () => void) {
  window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, listener);
  window.addEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, listener);
  return () => {
    window.removeEventListener(DEV_SIMULATE_UPDATE_EVENT, listener);
    window.removeEventListener(DEV_CLEAR_SIMULATED_UPDATE_EVENT, listener);
  };
}
