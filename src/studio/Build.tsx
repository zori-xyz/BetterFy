import {
  ArrowDown,
  ArrowUp,
  Check,
  CheckCheck,
  ChevronRight,
  CircleCheck,
  Gamepad2,
  Layers3,
  LoaderCircle,
  Plus,
  RotateCcw,
  Save,
  ShieldCheck,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import { useLocale, modCount } from "../i18n";
import {
  deliveryLabel,
  getConflicts,
  isPilotMod,
  isPilotSelection,
  modById,
  type StudioMod,
} from "./model";
import { Empty, PageHead } from "./ui";
import TreePilot from "./TreePilot";
import type { GameInstallation } from "../engine";

export type BuildPhase = "review" | "preparing" | "ready" | "restoring";
export type BuildState = {
  phase: BuildPhase;
  progress: number;
  completedIds: string[];
  startedAt: number | null;
};
export const initialBuild: BuildState = {
  phase: "review",
  progress: 0,
  completedIds: [],
  startedAt: null,
};
export default function Build({
  ids,
  variants,
  state,
  installation,
  preview,
  onRemove,
  onMove,
  onOpen,
  onCatalog,
  onStart,
  onCancel,
  onSave,
  onReset,
  onResolve,
  onPlay,
}: {
  ids: string[];
  variants: Record<string, number>;
  state: BuildState;
  installation: GameInstallation;
  preview: boolean;
  onRemove: (mod: StudioMod) => void;
  onMove: (id: string, direction: -1 | 1) => void;
  onOpen: (mod: StudioMod) => void;
  onCatalog: () => void;
  onStart: () => void;
  onCancel: () => void;
  onSave: () => void;
  onReset: () => void;
  onResolve: (keep: StudioMod, alternatives: StudioMod[]) => void;
  onPlay: () => void;
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
  const busy = state.phase === "preparing" || state.phase === "restoring";
  const ready = state.phase === "ready";
  const stages = isRu
    ? ["Проверяем состав", "Готовим моды", "Собираем профиль", "Завершаем подготовку"]
    : ["Reviewing selection", "Preparing mods", "Assembling profile", "Finishing up"];
  const step = Math.min(3, Math.floor(state.progress / 25));
  if (!chosen.length && !missing.length)
    return (
      <>
        <PageHead title={isRu ? "Моя сборка" : "My build"} />
        <Empty
          icon={<Layers3 />}
          title={isRu ? "Сначала выбери то, что нравится" : "Start with something you like"}
          text={
            isRu
              ? "Добавь облик или настройку из каталога. Здесь можно посмотреть весь набор и сценарий его подготовки."
              : "Add a look or game setting from the catalog. Review the selection and preview how preparation would work."
          }
          action={isRu ? "Открыть каталог" : "Open catalog"}
          onAction={onCatalog}
        />
        <TreePilot ids={[]} installation={installation} preview={preview} />
      </>
    );
  return (
    <div className="s-build">
      <PageHead
        eyebrow={
          ready
            ? isRu
              ? "ПРЕДПРОСМОТР ЗАВЕРШЁН"
              : "PREVIEW COMPLETE"
            : isRu
              ? "ТВОИ ИЗМЕНЕНИЯ"
              : "YOUR CHANGES"
        }
        title={
          ready ? (isRu ? "Выбор сохранён" : "Selection saved") : isRu ? "Моя сборка" : "My build"
        }
        description={
          ready
            ? isRu
              ? "Сценарий подготовки завершён. Файлы Dota 2 не изменялись."
              : "Preparation preview finished. Dota 2 files were not changed."
            : isRu
              ? "Выбранные облики и настройки. Проверь состав или добавь ещё."
              : "Your selected looks and settings. Review them or add more."
        }
      >
        <button className="s-btn" onClick={onSave} disabled={busy}>
          <Save />
          {isRu ? "Сохранить набор" : "Save build"}
        </button>
      </PageHead>
      <TreePilot ids={pilotIds} installation={installation} preview={preview} />
      {previewItems.length > 0 && (
        <div className="s-inline-note s-build-delivery-note">
          <TriangleAlert />
          <p>
            {isRu
              ? `${modCount(previewItems.length, language)} в этой сборке доступны только как превью. Они сохранятся в наборе, но не будут записаны в Dota 2.`
              : `${modCount(previewItems.length, language)} in this build ${previewItems.length === 1 ? "is" : "are"} preview-only. ${previewItems.length === 1 ? "It" : "They"} will stay in the build but will not be written to Dota 2.`}
          </p>
        </div>
      )}
      <div className="s-build-layout">
        <section className="s-build-list">
          <header>
            <strong>{modCount(chosen.length, language)}</strong>
            <button className="s-text-button" disabled={busy} onClick={onCatalog}>
              <Plus />
              {isRu ? "Добавить ещё" : "Add more"}
            </button>
          </header>
          {(["wardrobe", "game"] as const).map((domain) => {
            const list = chosen.filter((mod) => mod.domain === domain);
            return list.length ? (
              <div className="s-build-group" key={domain}>
                <h2>
                  {domain === "wardrobe"
                    ? isRu
                      ? "Облики и эффекты"
                      : "Looks & effects"
                    : isRu
                      ? "Приоритет настройки игры"
                      : "Game tuning priority"}
                  <span>{list.length}</span>
                  {domain === "game" && (
                    <small>
                      {isRu
                        ? "Первый мод имеет приоритет при совпадении файлов"
                        : "The first mod wins when files overlap"}
                    </small>
                  )}
                </h2>
                {list.map((mod, index) => (
                  <article className="s-build-item" key={mod.id}>
                    {domain === "game" && (
                      <span
                        className="s-build-priority"
                        title={isRu ? "Приоритет установки" : "Installation priority"}
                      >
                        {String(index + 1).padStart(2, "0")}
                      </span>
                    )}
                    <button
                      className="s-build-thumb s-build-mark"
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
                    <button
                      className="s-build-item-name"
                      onClick={() => onOpen(mod)}
                      disabled={busy}
                    >
                      <strong>{mod.name[language]}</strong>
                      <span>
                        {mod.categoryName[language]}
                        {mod.variants.length > 1 &&
                          ` · ${mod.variants[variants[mod.id] ?? 0]?.name}`}
                      </span>
                    </button>
                    {domain === "game" && list.length > 1 && (
                      <span className="s-build-order">
                        <button
                          className="s-icon"
                          disabled={busy || index === 0}
                          aria-label={`${isRu ? "Повысить приоритет" : "Increase priority"}: ${mod.name[language]}`}
                          onClick={() => onMove(mod.id, -1)}
                        >
                          <ArrowUp />
                        </button>
                        <button
                          className="s-icon"
                          disabled={busy || index === list.length - 1}
                          aria-label={`${isRu ? "Понизить приоритет" : "Decrease priority"}: ${mod.name[language]}`}
                          onClick={() => onMove(mod.id, 1)}
                        >
                          <ArrowDown />
                        </button>
                      </span>
                    )}
                    <span
                      className={`s-item-delivery ${isPilotMod(mod.id) ? "is-pilot" : "is-preview"}`}
                    >
                      {deliveryLabel(isPilotMod(mod.id) ? "pilot" : "preview", language)}
                    </span>
                    <span className={`s-item-state ${ready ? "ready" : ""}`}>
                      {ready ? (
                        <>
                          <Check />
                          {isRu ? "В наборе" : "Included"}
                        </>
                      ) : (
                        <>
                          <Plus />
                          {isRu ? "Добавлен" : "Added"}
                        </>
                      )}
                    </span>
                    <button
                      className="s-icon"
                      disabled={busy}
                      aria-label={`${isRu ? "Убрать" : "Remove"}: ${mod.name[language]}`}
                      onClick={() => onRemove(mod)}
                    >
                      <Trash2 />
                    </button>
                  </article>
                ))}
              </div>
            ) : null;
          })}
          {missing.length > 0 && (
            <div className="s-inline-note warning">
              <TriangleAlert />
              <p>
                {isRu
                  ? "Некоторые сохранённые моды отсутствуют в текущем каталоге. Обнови состав перед продолжением."
                  : "Some saved mods are missing from this catalog. Update your selection before continuing."}
              </p>
            </div>
          )}
          {conflicts.map((items, index) => (
            <section className="s-build-conflict" key={index}>
              <h3>
                <TriangleAlert />
                {isRu ? "Выбери один вариант" : "Choose one option"}
              </h3>
              <p>
                {isRu
                  ? "Эти моды занимают один слот. Остальные элементы набора сохранятся."
                  : "These mods share a slot. The rest of your build stays in place."}
              </p>
              {items.map((item) => (
                <button className="s-btn" key={item.id} onClick={() => onResolve(item, items)}>
                  {item.name[language]}
                  <Check />
                </button>
              ))}
            </section>
          ))}
        </section>
        {!verifiedPilotSelected && (
          <aside className={`s-build-summary ${ready ? "is-ready" : ""}`}>
            <span className="s-summary-icon">
              {busy ? <LoaderCircle className="s-spin" /> : ready ? <CheckCheck /> : <Layers3 />}
            </span>
            <h2>
              {busy
                ? state.phase === "restoring"
                  ? isRu
                    ? "Возвращаемся к выбору"
                    : "Returning to your selection"
                  : stages[step]
                : ready
                  ? isRu
                    ? "Предпросмотр завершён"
                    : "Preview complete"
                  : isRu
                    ? "Проверить набор"
                    : "Review your build"}
            </h2>
            <p>
              {busy
                ? isRu
                  ? "Выбор сохранён. Подготовку можно отменить."
                  : "Your selection is saved. You can cancel preparation."
                : ready
                  ? isRu
                    ? "Выбранные облики и настройки сохранены на этом устройстве."
                    : "Your selected looks and settings are saved on this device."
                  : isRu
                    ? "Посмотри сценарий подготовки. Это демонстрация: файлы игры не изменятся."
                    : "Preview the preparation flow. This is a demonstration: game files will not change."}
            </p>
            {busy && (
              <div
                className="s-build-progress"
                role="progressbar"
                aria-valuenow={state.progress}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label={isRu ? "Подготовка сборки" : "Build preparation"}
              >
                <div>
                  <i style={{ width: `${state.progress}%` }} />
                </div>
                <span>{state.progress}%</span>
              </div>
            )}
            <ol className="s-build-checklist">
              {stages.map((stage, index) => (
                <li
                  className={
                    ready || (busy && index < step)
                      ? "done"
                      : busy && index === step
                        ? "current"
                        : ""
                  }
                  key={stage}
                >
                  <span>{ready || (busy && index < step) ? <Check /> : index + 1}</span>
                  {stage}
                </li>
              ))}
            </ol>
            {busy ? (
              <button className="s-btn s-btn-full" onClick={onCancel}>
                {isRu ? "Отменить подготовку" : "Cancel preparation"}
              </button>
            ) : ready ? (
              <>
                <button className="s-btn s-btn-primary s-btn-full" onClick={onPlay}>
                  <Gamepad2 />
                  {isRu ? "Посмотреть запуск" : "Preview launch"}
                </button>
                <button className="s-text-button s-center" onClick={onReset}>
                  <RotateCcw />
                  {isRu ? "Вернуться к настройке" : "Return to editing"}
                </button>
              </>
            ) : (
              <>
                <button
                  className="s-btn s-btn-primary s-btn-full"
                  disabled={conflicts.length > 0 || missing.length > 0}
                  onClick={onStart}
                >
                  {isRu ? "Посмотреть подготовку" : "Preview preparation"}
                  <ChevronRight />
                </button>
                <small className="s-summary-hint">
                  {conflicts.length
                    ? isRu
                      ? "Сначала реши конфликты слотов"
                      : "Resolve slot conflicts first"
                    : isRu
                      ? "Без установки в Dota 2"
                      : "Without installing into Dota 2"}
                </small>
              </>
            )}
            <div className="s-summary-game">
              <Gamepad2 />
              <div>
                <strong>Dota 2</strong>
                <small>{isRu ? "Текущий игровой профиль" : "Current game profile"}</small>
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}
