import {
  ArrowDown,
  ArrowUp,
  Check,
  Gamepad2,
  Layers3,
  Plus,
  Save,
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
import { useEngineActive } from "./engineActivity";
import type { GameInstallation } from "../engine";

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
        eyebrow={isRu ? "ТВОИ ИЗМЕНЕНИЯ" : "YOUR CHANGES"}
        title={isRu ? "Моя сборка" : "My build"}
        description={
          isRu
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
                    <span className="s-item-state">
                      <Check />
                      {isRu ? "В наборе" : "Included"}
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
          <aside className="s-build-summary">
            <span className="s-summary-icon">
              <Layers3 />
            </span>
            <h2>{isRu ? "Установка пока недоступна" : "Installation not available yet"}</h2>
            <p>
              {isRu
                ? "Сейчас BetterFy записывает в Dota 2 только три пилотных мода и только на Windows. Остальной выбор хранится на этом устройстве — сохрани его в библиотеку, чтобы вернуться, когда установка откроется."
                : "Right now BetterFy writes only the three pilot mods to Dota 2, and only on Windows. The rest of your selection stays on this device — save it to your library to come back when installation opens."}
            </p>
            <button className="s-btn s-btn-primary s-btn-full" disabled={busy} onClick={onSave}>
              <Save />
              {isRu ? "Сохранить в библиотеку" : "Save to library"}
            </button>
            {conflicts.length > 0 && (
              <small className="s-summary-hint">
                {isRu ? "Реши конфликты слотов выше" : "Resolve the slot conflicts above"}
              </small>
            )}
            <div className="s-summary-game">
              <Gamepad2 />
              <div>
                <strong>Dota 2</strong>
                <small>{isRu ? "Файлы игры не изменены" : "Game files unchanged"}</small>
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}
