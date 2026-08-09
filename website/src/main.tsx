import React, { Component, type ErrorInfo, type ReactNode } from "react";
import ReactDOM from "react-dom/client";
import Site from "./Site";
import "./site.css";

class SiteErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(_error: Error, _info: ErrorInfo) {
    // The public fallback intentionally avoids collecting browser or account data.
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <main className="site-failure" role="alert">
        <div>
          <strong>BetterFy</strong>
          <h1>Страница не загрузилась</h1>
          <p>Обнови страницу. Если проблема повторится, открой официальный репозиторий.</p>
          <button onClick={() => window.location.reload()}>Обновить</button>
          <a href="https://github.com/zori-xyz/BetterFy">Открыть GitHub</a>
        </div>
      </main>
    );
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <SiteErrorBoundary><Site /></SiteErrorBoundary>
  </React.StrictMode>,
);
