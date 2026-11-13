import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

// Lets the frontend render in a plain browser (e.g. `pnpm dev:frontend`) where
// the native Tauri backend is absent. Data lives in memory only.
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

if (!isTauri) {
  const settings: Record<string, string> = {};

  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "get_all_settings":
          return { ...settings };
        case "get_setting":
          return settings[String(args.key)] ?? null;
        case "save_setting":
          settings[String(args.key)] = String(args.value);
          return null;
        case "delete_setting":
          delete settings[String(args.key)];
          return null;
        case "replace_all_settings":
          Object.keys(settings).forEach((key) => delete settings[key]);
          Object.assign(settings, args.settings as Record<string, string>);
          return null;
        case "get_current_db_path":
        case "get_settings_file_path":
          return "(browser preview — in-memory)";
      }
      if (cmd.startsWith("plugin:")) return null;
      const name = cmd.replace(/^[a-z]+_/, "");
      if (/(^|_)(list|recent|search|scan|read_all)/.test(cmd) || /^(get|load)_.*s$/.test(cmd) || name.endsWith("s")) {
        return [];
      }
      return null;
    },
    { shouldMockEvents: true },
  );
  console.info("[vora] Running without the Tauri backend: using in-memory browser mocks.");
}
