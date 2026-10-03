import { FlaskConical } from "lucide-react";
import { useLocale } from "../../i18n";
import { setDemoSettings, useDemoSettings, type DemoScenario } from "./demo";

// Controls for the browser preview: which way the simulated install goes,
// and whether the preview account is a developer.
export function DemoPanel() {
  const { isRu } = useLocale();
  const settings = useDemoSettings();
  const scenarios: Array<[DemoScenario, string, string]> = [
    ["success", "Всё проходит", "Everything works"],
    ["dota_open", "Dota не закрывается", "Dota will not close"],
    ["download_fails", "Обрыв загрузки", "Download drops"],
    ["steam_fails", "Steam меняет настройки", "Steam changes settings"],
  ];
  return (
    <div className="b-demo">
      <p>
        {isRu
          ? "Демо в браузере: шаги и тайминги изображены, файлы игры не меняются."
          : "Browser demo: steps and timings are simulated; no game files change."}
      </p>
      <label>
        <span>{isRu ? "Сценарий" : "Scenario"}</span>
        <select
          value={settings.scenario}
          onChange={(event) => setDemoSettings({ scenario: event.target.value as DemoScenario })}
        >
          {scenarios.map(([value, ru, en]) => (
            <option key={value} value={value}>
              {isRu ? ru : en}
            </option>
          ))}
        </select>
      </label>
      <label className="b-switch">
        <input
          type="checkbox"
          checked={settings.developer}
          onChange={(event) => setDemoSettings({ developer: event.target.checked })}
        />
        <i aria-hidden="true" />
        <FlaskConical />
        {isRu ? "Аккаунт разработчика" : "Developer account"}
      </label>
    </div>
  );
}
