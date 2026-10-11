import type { SectionId } from "@/features/settings/model/sections";

export const OPEN_SETTINGS_EVENT = "licht:open-settings";

export type OpenSettingsDetail = {
  section: SectionId;
};

export function requestOpenSettings(section: SectionId = "account") {
  window.dispatchEvent(
    new CustomEvent<OpenSettingsDetail>(OPEN_SETTINGS_EVENT, {
      detail: { section },
    }),
  );
}

export function onOpenSettingsRequest(
  handler: (section: SectionId) => void,
): () => void {
  const listener = (event: Event) => {
    const detail = (event as CustomEvent<OpenSettingsDetail>).detail;
    handler(detail?.section ?? "account");
  };
  window.addEventListener(OPEN_SETTINGS_EVENT, listener);
  return () => {
    window.removeEventListener(OPEN_SETTINGS_EVENT, listener);
  };
}
