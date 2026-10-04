import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Dices,
  Gamepad2,
  Layers3,
  Play,
  Plus,
  Save,
  Trash2,
  TriangleAlert,
  Check,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useLocale, modCount } from "../i18n";
import {
  deliveryLabel,
  getConflicts,
  isPilotMod,
  isPilotSelection,
  modById,
  type StudioMod,
} from "./model";
import { Empty } from "./ui";
import TreePilot, { type PilotState } from "./TreePilot";
import { useEngineActive } from "./engineActivity";
import type { GameInstallation } from "../engine";
import { CountUp } from "./delight";
import { useBuildName } from "./builder/buildName";
import { BuildJournal } from "./builder/BuildJournal";
import { journal } from "./builder/journal";
import { demoActive } from "./builder/demo";
import { findPackage } from "./packages";
import "./builder/builder.css";

const emptyLines: Array<[string, string, string, string]> = [
  [
    "Курьер вернулся пустым",
    "The courier came back empty-handed",
    "Добавь мод или облик из каталога — здесь они соберутся в одну сборку.",
    "Add a mod or a look from the catalog and it lands here, in one build.",
  ],
  [
    "Пусто, как линия без крипов",
    "Empty as a lane without creeps",
    "Выбери, что поменять в Dota: звук, карту, HUD или облики героев.",
    "Pick what to change in Dota: audio, map, HUD or hero looks.",
  ],
  [
    "Ни одного мода. Даже веточки нет",
    "Not a single mod. Not even an Iron Branch",
    "Начни с каталога: первый мод сразу получит место в сборке.",
    "Start in the catalog: the first mod gets its slot right away.",
  ],
];

// Mods whose files are replaced with silent blanks.
const silencing = (id: string) => /mute|remove-pings/.test(id);

function launchDota() {
  if (demoActive()) {
    journal.log(
      "ok",
      "Демо: здесь Steam запустил бы Dota 2.",
      "Demo: this is where Steam would start Dota 2.",
    );
    return;
  }
  journal.log("step", "Просим Steam запустить Dota 2.", "Asking Steam to start Dota 2.");
  openUrl("steam://rungameid/570").catch(() =>
    journal.log(
      "warn",
      "Steam не открыл Dota 2 — запусти её из библиотеки Steam.",
      "Steam did not open Dota 2. Start it from your Steam library.",
      "steam_url_failed",
    ),
  );
}

export default function Build({
  ids,
  variants,
  installation,
  preview,
  onRemove,
  onMove,
  onOpen,
  onCatalog,
  onSave,
  onResolve,
  onRemoveMissing,
}: {
  ids: string[];
  variants: Record<string, number>;
  installation: GameInstallation;
  preview: boolean;
  onRemove: (mod: StudioMod) => void;
  onMove: (id: string, direction: -1 | 1) => void;
  onOpen: (mod: StudioMod) => void;
  onCatalog: () => void;
  onSave: () => void;
  onResolve: (keep: StudioMod, alternatives: StudioMod[]) => void;
  onRemoveMissing: () => void;
}) {
  const { language, isRu } = useLocale();
  const chosen = ids.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
  const missing = ids.filter((id) => !modById.has(id));
  const conflicts = getConflicts(ids);
  const gameIds = chosen.filter((mod) => mod.domain === "game").map((mod) => mod.id);
  const verifiedPilotSelected = isPilotSelection(gameIds);
  // The pilot subset is installable even when other game mods are selected;
  // those stay preview-only and are listed in the note below.
  const pilotIds = gameIds.filter(isPilotMod);
  const previewItems = chosen.filter((mod) => mod.domain === "wardrobe" || !isPilotMod(mod.id));
  const busy = useEngineActive();
  const name = useBuildName(ids, language);
  const [pilot, setPilot] = useState<PilotState | null>(null);
  const [folding, setFolding] = useState(false);
  const [showList, setShowList] = useState(false);
  const empty = useMemo(() => emptyLines[Math.floor(Math.random() * emptyLines.length)], []);
  const wasInstalled = useRef(false);
  // The finished build folds the list into one sealed card, once.
  useEffect(() => {
    if (pilot?.justInstalled && !wasInstalled.current) {
      setFolding(true);
      setShowList(false);
      const timer = window.setTimeout(() => setFolding(false), 760);
      wasInstalled.current = true;
      return () => window.clearTimeout(timer);
    }
    if (!pilot?.installed) wasInstalled.current = false;
  }, [pilot?.justInstalled, pilot?.installed]);
  const installed = Boolean(pilot?.installed);
  const plan = pilot?.plan ?? null;
  const muted = plan
    ? plan.contributions
        .filter((item) => silencing(item.packageId))
        .reduce((sum, item) => sum + item.effectiveResources, 0)
    : 0;
  const installedMods = (pilot?.installedPackageIds ?? []).flatMap((packageId) => {
    const catalogId = findPackage(packageId)?.catalogId;
    return catalogId && modById.has(catalogId) ? [modById.get(catalogId)!] : [];
  });

  if (!chosen.length && !missing.length)
    return (
      <div className="b-build is-empty">
        <Empty
          icon={<Layers3 />}
          title={isRu ? empty[0] : empty[1]}
          text={isRu ? empty[2] : empty[3]}
          action={isRu ? "Открыть каталог" : "Open catalog"}
          onAction={onCatalog}
        />
        <TreePilot ids={[]} installation={installation} preview={preview} onState={setPilot} />
      </div>
    );

  const stats: Array<{ label: string; value: number | null; hint?: string; tone?: string }> = [
    { label: isRu ? "Модов в сборке" : "Mods in build", value: chosen.length },
    {
      label: isRu ? "Ставятся в Dota" : "Go into Dota",
      value: pilotIds.length,
      hint: previewItems.length
        ? isRu
          ? `ещё ${previewItems.length} только превью`
          : `${previewItems.length} more preview-only`
        : undefined,
    },
    {
      label: isRu ? "Файлов в сборке" : "Files in build",
      value: plan?.resourceCount ?? null,
      hint: plan ? undefined : isRu ? "после проверки" : "after checking",
    },
    muted > 0
      ? { label: isRu ? "Звуков заглушено" : "Sounds muted", value: muted, tone: "quiet" }
      : {
          label: isRu ? "Совпадений решено" : "Overlaps resolved",
          value: plan ? plan.duplicateResources + plan.overriddenResources : null,
          hint: plan ? undefined : isRu ? "после проверки" : "after checking",
        },
  ];

  const list = (["game", "wardrobe"] as const).map((domain) => {
    const items = chosen.filter((mod) => mod.domain === domain);
    return items.length ? (
      <div className="b-group" key={domain}>
        <h2>
          {domain === "wardrobe"
            ? isRu
              ? "Облики и эффекты"
              : "Looks & effects"
            : isRu
              ? "Настройки игры"
              : "Game tuning"}
          <span>{items.length}</span>
          {domain === "game" && items.length > 1 && (
            <small>
              {isRu ? "Верхний мод главнее при совпадении файлов" : "The top mod wins on overlaps"}
            </small>
          )}
        </h2>
        <ol>
          {items.map((mod, index) => (
            <li
              className="b-row"
              key={mod.id}
              style={{ ["--i" as string]: index }}
              data-delivery={isPilotMod(mod.id) ? "pilot" : "preview"}
            >
              {domain === "game" && (
                <span className="b-row-priority" title={isRu ? "Приоритет" : "Priority"}>
                  {String(index + 1).padStart(2, "0")}
                </span>
              )}
              <button
                className="b-row-thumb"
                aria-label={`${isRu ? "Подробнее" : "Details"}: ${mod.name[language]}`}
                onClick={() => onOpen(mod)}
                disabled={busy}
              >
                {mod.image ? (
                  <img src={mod.image} alt="" loading="lazy" />
                ) : (
                  <span>{mod.domain === "game" ? <Gamepad2 /> : <Layers3 />}</span>
                )}
              </button>
              <button className="b-row-name" onClick={() => onOpen(mod)} disabled={busy}>
                <strong>{mod.name[language]}</strong>
                <span>
                  {mod.categoryName[language]}
                  {mod.variants.length > 1 && ` · ${mod.variants[variants[mod.id] ?? 0]?.name}`}
                </span>
              </button>
              <span className={`b-row-flag ${isPilotMod(mod.id) ? "is-pilot" : "is-preview"}`}>
                {deliveryLabel(isPilotMod(mod.id) ? "pilot" : "preview", language)}
              </span>
              {domain === "game" && items.length > 1 && (
                <span className="b-row-order">
                  <button
                    className="s-icon"
                    disabled={busy || installed || index === 0}
                    aria-label={`${isRu ? "Выше" : "Move up"}: ${mod.name[language]}`}
                    onClick={() => onMove(mod.id, -1)}
                  >
                    <ArrowUp />
                  </button>
                  <button
                    className="s-icon"
                    disabled={busy || installed || index === items.length - 1}
                    aria-label={`${isRu ? "Ниже" : "Move down"}: ${mod.name[language]}`}
                    onClick={() => onMove(mod.id, 1)}
                  >
                    <ArrowDown />
                  </button>
                </span>
              )}
              <button
                className="s-icon b-row-remove"
                disabled={busy}
                aria-label={`${isRu ? "Убрать" : "Remove"}: ${mod.name[language]}`}
                onClick={() => {
                  journal.log(
                    "step",
                    `Убран из сборки: ${mod.name.ru}.`,
                    `Removed from the build: ${mod.name.en}.`,
                  );
                  onRemove(mod);
                }}
              >
                <Trash2 />
              </button>
            </li>
          ))}
        </ol>
      </div>
    ) : null;
  });

  return (
    <div className={`b-build ${installed ? "is-installed" : ""} ${folding ? "is-folding" : ""}`}>
      <header className="b-hero">
        <div>
          <span className="s-eyebrow">
            {isRu ? "СБОРКА" : "BUILD"} · {modCount(chosen.length, language).toUpperCase()}
            {name.summary && ` · ${name.summary.toUpperCase()}`}
          </span>
          <h1>
            <span key={name.name} className="b-name">
              {name.name}
            </span>
            {name.canRoll && (
              <button
                className="s-icon b-roll"
                type="button"
                onClick={name.roll}
                title={isRu ? "Другое имя" : "Another name"}
                aria-label={isRu ? "Придумать другое имя сборке" : "Give the build another name"}
              >
                <Dices />
              </button>
            )}
          </h1>
          <p>
            {installed
              ? isRu
                ? "Сборка стоит в Dota 2. Откатить можно в любой момент."
                : "The build is in Dota 2. You can restore the game any time."
              : verifiedPilotSelected
                ? isRu
                  ? "Путь в игру: проверка файлов, запись с бэкапом, Steam. Откат — в один клик."
                  : "The way into the game: check files, write with a backup, Steam. Restore in one click."
                : isRu
                  ? "Имя придумано по составу. Нажми на кубик, если не нравится."
                  : "Named after what is inside. Roll the dice if you want another."}
          </p>
        </div>
        <div className="s-head-actions">
          <button className="s-btn" onClick={onSave} disabled={busy}>
            <Save />
            {isRu ? "Сохранить набор" : "Save build"}
          </button>
        </div>
      </header>

      <dl className="b-stats">
        {stats.map((stat) => (
          <div key={stat.label} className={stat.tone ? `is-${stat.tone}` : ""}>
            <dt>{stat.label}</dt>
            <dd>
              {stat.value === null ? (
                <span className="b-stat-pending">—</span>
              ) : (
                <CountUp value={stat.value} locale={isRu ? "ru-RU" : "en-US"} />
              )}
            </dd>
            {stat.hint && <small>{stat.hint}</small>}
          </div>
        ))}
      </dl>

      <div className="b-layout">
        <section className="b-composition" aria-label={isRu ? "Состав сборки" : "Build contents"}>
          {installed && (
            <article className="b-sealed">
              <div className="b-sealed-stack" aria-hidden="true">
                {(installedMods.length ? installedMods : chosen).slice(0, 4).map((mod, index) => (
                  <span key={mod.id} style={{ ["--i" as string]: index }}>
                    {mod.image ? <img src={mod.image} alt="" /> : <Gamepad2 />}
                  </span>
                ))}
              </div>
              <div className="b-sealed-copy">
                <small>
                  <Check />
                  {isRu ? "ГОТОВО" : "DONE"}
                </small>
                <h2>{isRu ? "Готово. Запускай Dota" : "Done. Go play Dota"}</h2>
                <p>
                  {isRu
                    ? `${modCount(installedMods.length || pilotIds.length, language)} в одном файле${pilot?.installedLanguage ? ` · -language ${pilot.installedLanguage}` : ""}`
                    : `${modCount(installedMods.length || pilotIds.length, language)} in one file${pilot?.installedLanguage ? ` · -language ${pilot.installedLanguage}` : ""}`}
                </p>
              </div>
              <button className="s-btn s-btn-primary b-play" disabled={busy} onClick={launchDota}>
                <Play />
                {isRu ? "Запустить Dota 2" : "Launch Dota 2"}
              </button>
              <button
                className="s-text-button b-sealed-toggle"
                type="button"
                aria-expanded={showList}
                onClick={() => setShowList((value) => !value)}
              >
                {showList
                  ? isRu
                    ? "Скрыть состав"
                    : "Hide contents"
                  : isRu
                    ? "Показать состав"
                    : "Show contents"}
              </button>
            </article>
          )}
          {(!installed || showList || folding) && (
            <div className="b-list">
              <header>
                <strong>{isRu ? "Состав" : "Contents"}</strong>
                <button className="s-text-button" disabled={busy} onClick={onCatalog}>
                  <Plus />
                  {isRu ? "Добавить ещё" : "Add more"}
                </button>
              </header>
              {list}
              {previewItems.length > 0 && (
                <p className="b-list-note">
                  {isRu
                    ? `${modCount(previewItems.length, language)} пока только превью: они останутся в наборе, но в Dota не запишутся.`
                    : `${modCount(previewItems.length, language)} ${previewItems.length === 1 ? "is" : "are"} preview-only for now: kept in the build, not written to Dota.`}
                </p>
              )}
              {missing.length > 0 && (
                <div className="s-inline-note warning">
                  <TriangleAlert />
                  <p>
                    {isRu
                      ? `${modCount(missing.length, language)} из сохранённого выбора больше нет в каталоге.`
                      : `${modCount(missing.length, language)} from your saved selection ${missing.length === 1 ? "is" : "are"} no longer in the catalog.`}
                  </p>
                  <button className="s-btn" disabled={busy} onClick={onRemoveMissing}>
                    <Trash2 />
                    {isRu ? "Убрать отсутствующие" : "Remove missing"}
                  </button>
                </div>
              )}
              {conflicts.map((items, index) => (
                <section className="b-conflict" key={index}>
                  <h3>
                    <TriangleAlert />
                    {isRu ? "Два мода на один слот" : "Two mods, one slot"}
                  </h3>
                  <p>
                    {isRu
                      ? "Оставь один — остальное в сборке не изменится."
                      : "Keep one; the rest of the build stays as is."}
                  </p>
                  <div>
                    {items.map((item) => (
                      <button
                        className="s-btn"
                        key={item.id}
                        onClick={() => onResolve(item, items)}
                      >
                        {isRu ? "Оставить" : "Keep"} {item.name[language]}
                      </button>
                    ))}
                  </div>
                </section>
              ))}
            </div>
          )}
        </section>
        <aside className="b-side">
          {pilotIds.length > 0 || installed ? null : (
            <div className="b-side-note">
              <span>
                <Layers3 />
              </span>
              <h2>{isRu ? "В Dota пока нечего ставить" : "Nothing to install yet"}</h2>
              <p>
                {isRu
                  ? "Сейчас BetterFy записывает в игру только моды с отметкой «Windows-пилот». Остальное сохрани в библиотеку — вернёшься, когда установка откроется."
                  : "Right now BetterFy only writes mods marked Windows pilot. Save the rest to your library and come back when installation opens."}
              </p>
              <button className="s-btn s-btn-primary s-btn-full" disabled={busy} onClick={onSave}>
                <Save />
                {isRu ? "Сохранить в библиотеку" : "Save to library"}
              </button>
              {conflicts.length > 0 && (
                <small>
                  {isRu ? "Сначала реши конфликт слотов" : "Resolve the slot conflict first"}
                </small>
              )}
            </div>
          )}
          <TreePilot
            ids={pilotIds}
            installation={installation}
            preview={preview}
            onState={setPilot}
          />
        </aside>
      </div>
      <BuildJournal busy={busy || Boolean(pilot?.busy)} developer={Boolean(pilot?.developer)} />
    </div>
  );
}
