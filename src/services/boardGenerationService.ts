import { invoke } from "@tauri-apps/api/core";

export const boardGenerationService = {
  saveApiKey(apiKey: string): Promise<void> {
    return invoke<void>("save_deepseek_key", { apiKey });
  },

  generate(prompt: string): Promise<void> {
    return invoke<void>("generate_board_from_prompt", { prompt });
  },
};
