import { Modal } from "./ui";
import { useLocale } from "../i18n";

const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);

// The keyboard map: one place that says what the keys do, opened with "?".
export default function Shortcuts({ onClose }: { onClose: () => void }) {
  const { isRu } = useLocale();
  const mod = isMac ? "⌘" : "Ctrl";
  const rows: { keys: string[]; label: string }[] = [
    { keys: [mod, "K"], label: isRu ? "Найти мод" : "Find a mod" },
    { keys: ["G", "H"], label: isRu ? "Обзор" : "Overview" },
    { keys: ["G", "C"], label: isRu ? "Каталог" : "Discover" },
    { keys: ["G", "B"], label: isRu ? "Моя сборка" : "My build" },
    { keys: ["G", "L"], label: isRu ? "Библиотека" : "Library" },
    { keys: ["G", "S"], label: isRu ? "Настройки" : "Settings" },
    { keys: ["?"], label: isRu ? "Эта шпаргалка" : "This cheat sheet" },
    { keys: ["Esc"], label: isRu ? "Закрыть окно" : "Close a window" },
  ];
  return (
    <Modal title={isRu ? "Горячие клавиши" : "Keyboard shortcuts"} onClose={onClose}>
      <div className="s-dialog-body s-shortcuts">
        <p>
          {isRu
            ? "Нажми G, затем букву: так быстрее, чем тянуться мышкой."
            : "Press G, then a letter: faster than reaching for the mouse."}
        </p>
        <ul>
          {rows.map((row) => (
            <li key={row.label}>
              <span>{row.label}</span>
              <span className="s-shortcut-keys">
                {row.keys.map((key, index) => (
                  <span key={key}>
                    {index > 0 && row.keys[0] === "G" && <i>{isRu ? "потом" : "then"}</i>}
                    <kbd>{key}</kbd>
                  </span>
                ))}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </Modal>
  );
}
