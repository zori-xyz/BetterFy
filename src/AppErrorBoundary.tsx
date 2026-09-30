import { Component, type ErrorInfo, type ReactNode } from "react";
import BetterFyWordmark from "./BetterFyWordmark";

type State = { failed: boolean };

export default class AppErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[BetterFy] UI recovery boundary", error, info.componentStack);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    const isRu = document.documentElement.lang !== "en";

    return (
      <main className="app-recovery-boundary">
        <div className="app-recovery-card">
          <BetterFyWordmark hero />
          <span>BETTERFY / SAFE MODE</span>
          <h1>{isRu ? "Перезапустим интерфейс BetterFy" : "Restart the BetterFy interface"}</h1>
          <p>
            {isRu
              ? "Произошла ошибка интерфейса. Установки и откаты выполняются отдельно от интерфейса и ведут журнал — после перезапуска проверь состояние сборки на экране «Моя сборка»."
              : "The interface encountered an error. Installs and restores run separately from the interface and are journaled — after restarting, check your build on the My build screen."}
          </p>
          <button onClick={() => window.location.reload()}>
            {isRu ? "Перезапустить BetterFy" : "Restart BetterFy"}
          </button>
        </div>
      </main>
    );
  }
}
