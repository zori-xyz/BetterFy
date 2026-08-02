import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/manrope/400.css";
import "@fontsource/manrope/500.css";
import "@fontsource/manrope/600.css";
import "@fontsource/manrope/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "./styles.css";
import App from "./App";
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
