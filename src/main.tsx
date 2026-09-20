import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/manrope/latin-400.css";
import "@fontsource/manrope/cyrillic-400.css";
import "@fontsource/manrope/latin-500.css";
import "@fontsource/manrope/cyrillic-500.css";
import "@fontsource/manrope/latin-600.css";
import "@fontsource/manrope/cyrillic-600.css";
import "@fontsource/manrope/latin-700.css";
import "@fontsource/manrope/cyrillic-700.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/cyrillic-400.css";
import "./studio/studio.css";
import "./studio/studio-polish.css";
import "./studio/studio-finish.css";
import "./studio/auth-id.css";
import App from "./studio/StudioApp";
import AppErrorBoundary from "./AppErrorBoundary";
import { LocaleProvider } from "./i18n";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppErrorBoundary>
      <LocaleProvider>
        <App />
      </LocaleProvider>
    </AppErrorBoundary>
  </React.StrictMode>,
);
