import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

/** Default route is always Home. Chart remains available only as an explicit route, not as the app default. */
export type DefaultRoute = "home" | "chart";

const KEY = "vora.default_route";

export function getDefaultRoute(): DefaultRoute {
  void getSetting(KEY);
  return "home";
}

export function setDefaultRoute(route: DefaultRoute): void {
  setSetting(KEY, route === "chart" ? "home" : "home");
}
