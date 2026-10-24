import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route, Navigate, useLocation } from "react-router-dom";
import App from "./App";
import { SourcesPage } from "./pages/SourcesPage";
import { ChartPage } from "./pages/ChartPage";
import { SnapshotsPage } from "./pages/SnapshotsPage";
import { getDefaultRoute } from "./lib/settings";
import { migrateLegacySnapshots } from "./lib/snapshot/storage";
import { initializeSettingsStore } from "./shared/store/useSettingsStore";
import { useAppStore } from "./shared/store/useAppStore";
import "./App.css";
import "./features/markdown-editor/styles/editor.css";
import "./features/markdown-editor/styles/markdown-preview.css";
import "./features/markdown-editor/styles/callouts.css";

function ScaleLabRedirect(): React.ReactElement {
  const location = useLocation();
  return (
    <Navigate
      to={{ pathname: "/chart", search: location.search, hash: location.hash }}
      replace
    />
  );
}

function AppRoutes(): React.ReactElement {
  const location = useLocation();
  const chartVisible = location.pathname === "/chart";
  const snapshotsVisible = location.pathname === "/snapshots";
  const keepChartMounted = chartVisible || snapshotsVisible;

  return (
    <>
      {keepChartMounted && (
        <div
          aria-hidden={!chartVisible}
          style={{
            position: "fixed",
            inset: 0,
            visibility: chartVisible ? "visible" : "hidden",
            pointerEvents: chartVisible ? "auto" : "none",
          }}
        >
          <ChartPage isVisible={chartVisible} />
        </div>
      )}
      <Routes>
        <Route
          path="/"
          element={defaultRoute === "chart" ? <Navigate to="/chart" replace /> : <App />}
        />
        <Route path="/home" element={<App />} />
        <Route path="/market" element={<Navigate to="/chart" replace />} />
        <Route path="/sources" element={<SourcesPage />} />
        <Route path="/chart" element={null} />
        <Route path="/scale-lab" element={<ScaleLabRedirect />} />
        <Route path="/snapshots" element={snapshotsVisible ? <SnapshotsPage /> : null} />
      </Routes>
    </>
  );
}

let defaultRoute: ReturnType<typeof getDefaultRoute> = "chart";

async function bootstrap(): Promise<void> {
  try {
    const settings = await initializeSettingsStore();
    useAppStore.setState({
      activeBoardId: settings["vora.canvas.active_board_id"] ?? "",
      textDirection: settings["vora.ui.text_direction"] === "ltr" ? "ltr" : "rtl",
    });
    defaultRoute = getDefaultRoute();
    await migrateLegacySnapshots();
  } catch (error: unknown) {
    console.error("Could not initialize database-backed application state:", error);
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <BrowserRouter>
        <AppRoutes />
      </BrowserRouter>
    </React.StrictMode>,
  );
}

void bootstrap();
