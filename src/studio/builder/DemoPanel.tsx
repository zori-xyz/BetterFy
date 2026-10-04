import { useId } from "react";
import { FlaskConical } from "lucide-react";
import { useLocale } from "../../i18n";
import { ChoiceCards } from "./Choice";
import { setDemoSettings, useDemoSettings, type DemoScenario } from "./demo";

// Controls for the browser preview: which way the simulated install goes,
// and whether the preview account is a developer.
export function DemoPanel() {
  const { isRu } = useLocale();
  const settings = useDemoSettings();
  const labelId = useId();
  // A failing scenario names the engine code the demo bridge throws for it.
  const scenarios: Array<[DemoScenario, string, string, string?]> = [
    ["success", "Всё проходит", "Everything works"],
    ["dota_open", "Dota не закрывается", "Dota will not close", "runtime_busy"],
    ["download_fails", "Обрыв загрузки", "Download drops", "download_transport_failed"],
    ["steam_fails", "Steam меняет настройки", "Steam changes settings", "steam_config_plan_stale"],
  ];
  return (
    <div className="b-demo">
      <p>
        {isRu
          ? "Демо в браузере: шаги и тайминги изображены, файлы игры не меняются."
          : "Browser demo: steps and timings are simulated; no game files change."}
      </p>
      <div className="b-demo-field">
        <span id={labelId}>{isRu ? "Сценарий" : "Scenario"}</span>
        <ChoiceCards
          size="chip"
          label={isRu ? "Сценарий демо" : "Demo scenario"}
          labelledBy={labelId}
          value={settings.scenario}
          options={scenarios.map(([value, ru, en, code]) => ({
            value,
            label: isRu ? ru : en,
            meta: code,
          }))}
          onChange={(scenario) => setDemoSettings({ scenario })}
        />
      </div>
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
