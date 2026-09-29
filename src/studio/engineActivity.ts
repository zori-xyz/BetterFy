import { useEffect, useState } from "react";

// True while BetterFy is downloading, installing, restoring or recovering game
// files. Anything that would interrupt that work — the updater restarting the
// app, switching the game folder, changing the selected mods — checks it.
let active = false;
const listeners = new Set<(value: boolean) => void>();

export function setEngineActive(value: boolean) {
  if (value === active) return;
  active = value;
  listeners.forEach((listener) => listener(value));
}

export function useEngineActive() {
  const [value, setValue] = useState(active);
  useEffect(() => {
    listeners.add(setValue);
    setValue(active);
    return () => {
      listeners.delete(setValue);
    };
  }, []);
  return value;
}
