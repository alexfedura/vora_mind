import { log } from "./logger";
import { configureGitHubUpdater } from "./github-updater";

export function configureUpdater(): void {
  configureGitHubUpdater(log);
}
