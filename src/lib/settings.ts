import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

/** v7.0.0 Part C: default-route setting (persisted, default: chart). */
export type DefaultRoute = "home" | "chart";

const KEY = "vora.default_route";

export function getDefaultRoute(): DefaultRoute {
  const value = getSetting(KEY);
  return value === "home" ? "home" : "chart";
}

export function setDefaultRoute(route: DefaultRoute): void {
  setSetting(KEY, route);
}
