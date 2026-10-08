/** Theme handling: light / dark / system. CSS does the actual work via the
 *  `data-theme` attribute + a prefers-color-scheme media query. */

import { api } from "../../api";
import type { ThemePref } from "../../db/models";

export function applyThemeToDom(pref: ThemePref): void {
  document.documentElement.setAttribute("data-theme", pref);
}

/** Apply immediately and persist (best-effort — theme is not critical data). */
export async function setTheme(pref: ThemePref): Promise<void> {
  applyThemeToDom(pref);
  try {
    await api.setTheme(pref);
  } catch {
    /* ignore persistence errors */
  }
}
