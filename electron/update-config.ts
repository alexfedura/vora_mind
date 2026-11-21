export interface GitHubUpdateFeed {
  provider: "github";
  owner: string;
  repo: string;
  private: false;
  channel: "latest";
}

export function getGitHubUpdateFeed(): GitHubUpdateFeed {
  return {
    provider: "github",
    owner: "alexfedura",
    repo: "vora_mind",
    private: false,
    channel: "latest"
  };
}
