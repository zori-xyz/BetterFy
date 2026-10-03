import { useEffect, useState } from "react";

// One line under the current step while BetterFy works. Each line names
// something BetterFy really does in that phase, with a little Dota on top.

type Phrase = [ru: string, en: string];

const phrases: Record<"download" | "install" | "steam" | "restore", Phrase[]> = {
  download: [
    ["Курьер уже несёт файлы модов", "The courier is bringing the mod files"],
    ["Сверяем каждый файл по SHA-256", "Checking every file against its SHA-256"],
    ["Непроверенный файл в сборку не попадёт", "No unchecked file gets into the build"],
    ["Сверяемся с файлами твоей Dota", "Matching against your Dota files"],
  ],
  install: [
    ["Мягко закрываем Dota и Steam", "Closing Dota and Steam gracefully"],
    ["Кладём исходный файл в бэкап", "Putting the original file into a backup"],
    ["Собираем моды в один файл", "Combining the mods into one file"],
    ["Пишем так, чтобы можно было откатить", "Writing it so it can be rolled back"],
  ],
  steam: [
    ["Добавляем -language в профиль Steam", "Adding -language to the Steam profile"],
    ["Будим Steam", "Waking Steam up"],
  ],
  restore: [
    ["Возвращаем всё как было", "Putting everything back"],
    ["Сверяем исходный файл с бэкапом", "Checking the original file against the backup"],
    ["Телепорт на базу", "Teleporting to base"],
  ],
};

export type PhrasePhase = keyof typeof phrases;

function motionAllowed() {
  return (
    !document.documentElement.classList.contains("motion-disabled") &&
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/** The current line for a phase; rotates every few seconds while it lasts. */
export function usePhrase(phase: PhrasePhase | null, isRu: boolean) {
  const [index, setIndex] = useState(0);
  useEffect(() => {
    setIndex(0);
    if (!phase || !motionAllowed()) return;
    const timer = window.setInterval(() => setIndex((value) => value + 1), 2800);
    return () => window.clearInterval(timer);
  }, [phase]);
  if (!phase) return null;
  const list = phrases[phase];
  const phrase = list[index % list.length];
  return { key: `${phase}-${index % list.length}`, text: phrase[isRu ? 0 : 1] };
}
