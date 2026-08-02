import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { getStorageItem, setStorageItem } from "./storage";

export type Language = "ru" | "en";

type LocaleContextValue = {
  language: Language;
  setLanguage: (language: Language) => void;
  isRu: boolean;
};

const LocaleContext = createContext<LocaleContextValue | null>(null);

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [language, setLanguage] = useState<Language>(() => {
    const stored = getStorageItem("betterfy:language");
    return stored === "en" ? "en" : "ru";
  });

  useEffect(() => {
    setStorageItem("betterfy:language", language);
    document.documentElement.lang = language;
  }, [language]);

  const value = useMemo(
    () => ({ language, setLanguage, isRu: language === "ru" }),
    [language],
  );

  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>;
}

export function useLocale() {
  const context = useContext(LocaleContext);
  if (!context) throw new Error("useLocale must be used inside LocaleProvider");
  return context;
}

export function modCount(count: number, language: Language) {
  if (language === "en") return `${count} ${count === 1 ? "mod" : "mods"}`;
  const mod10 = count % 10;
  const mod100 = count % 100;
  const word =
    mod10 === 1 && mod100 !== 11
      ? "мод"
      : mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)
        ? "мода"
        : "модов";
  return `${count} ${word}`;
}
