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
          <h1>{isRu ? "Вернём интерфейс в спокойное состояние" : "Let’s return the interface to a calm state"}</h1>
          <p>
            {isRu
              ? "Произошла ошибка интерфейса. Игровые файлы не затронуты — перезапусти оболочку и продолжай."
              : "The interface encountered an error. Game files are untouched — restart the shell and continue."}
          </p>
          <button onClick={() => window.location.reload()}>
            {isRu ? "Перезапустить BetterFy" : "Restart BetterFy"}
          </button>
        </div>
      </main>
    );
  }
}
