import type { LayoutStorage } from "react-resizable-panels";

/** Default sidebar width in logical pixels. */
export const SIDEBAR_DEFAULT_SIZE_PX = 320;
export const SIDEBAR_MIN_SIZE_PX = 320;
export const SIDEBAR_MAX_SIZE_PX = 420;

export const SIDEBAR_LAYOUT_ID = "instances-sidebar";
export const SIDEBAR_PANEL_ID = "sidebar";
export const MAIN_PANEL_ID = "main";

export const SIDEBAR_PANEL_IDS = [SIDEBAR_PANEL_ID, MAIN_PANEL_ID] as const;

/** localStorage wrapper that never throws (private mode / quota). */
export const sidebarLayoutStorage: LayoutStorage = {
  getItem(name) {
    try {
      return localStorage.getItem(name);
    } catch {
      return null;
    }
  },
  setItem(name, value) {
    try {
      localStorage.setItem(name, value);
    } catch {
      // Ignore persistence failures; resize still works for the session.
    }
  },
};
