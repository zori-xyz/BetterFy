import { useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  Braces,
  Check,
  ExternalLink,
  FileCode2,
  Files,
  Gauge,
  ImageOff,
  Layers3,
  ListX,
  Plus,
  Search,
  ShieldQuestion,
} from "lucide-react";
import type { Language } from "./i18n";
import AccentTitle from "./AccentTitle";
import { getStoredStringArray, setStorageItem } from "./storage";
import {
  minifyCategoryLabels,
  minifyMods,
  minifyPreviewUrl,
  minifySource,
  type MinifyCatalogItem,
  type MinifyCategory,
} from "./minifyCatalog";

const copy = {
  ru: {
    eyebrow: "MINIFY / GAME MODS",
    title: "Убери лишнее. Оставь игру.",
    text: "Функциональные моды из официального Dota2 Minify: оптимизация эффектов, интерфейса, звука и игровых инструментов.",
    selected: "В СБОРКЕ",
    selectedEmpty: "Выбранные Minify-моды появятся здесь.",
    prototype: "Выбор сохраняется локально. Реальный патчинг Minify ещё не подключён.",
    search: "Найти мод Minify",
    add: "Добавить в сборку",
    remove: "Убрать из сборки",
    inBuild: "В сборке",
    source: "Открыть источник",
    sourceLabel: "ОФИЦИАЛЬНЫЙ MINIFY",
    unknownPreview: "В исходном репозитории нет превью",
    blacklist: "Удаляемых ресурсов",
    files: "Файлов замены",
    styles: "Строк стилей",
    scripts: "Patch-скриптов",
    compatibility: "Совместимость с текущей Dota не проверена BetterFy",
    results: "модов в официальном составе",
    empty: "Ничего не найдено",
    emptyText: "Сбрось фильтры или выбери другую категорию.",
    reset: "Сбросить фильтры",
    selectedOnly: "Только выбранные",
    details: "Открыть детали",
    noEvidence: "Состав определяется конфигурацией Minify",
  },
  en: {
    eyebrow: "MINIFY / GAME MODS",
    title: "Remove the noise. Keep the game.",
    text: "Functional mods from the official Dota2 Minify project: effect optimization, interface, audio, and game utilities.",
    selected: "IN BUILD",
    selectedEmpty: "Selected Minify mods will appear here.",
    prototype: "Selection is stored locally. Real Minify patching is not connected yet.",
    search: "Find a Minify mod",
    add: "Add to build",
    remove: "Remove from build",
    inBuild: "In build",
    source: "Open source",
    sourceLabel: "OFFICIAL MINIFY",
    unknownPreview: "No preview in the source repository",
    blacklist: "Removed resources",
    files: "Replacement files",
    styles: "Style lines",
    scripts: "Patch scripts",
    compatibility: "Compatibility with the current Dota build is not verified by BetterFy",
    results: "mods in the official set",
    empty: "Nothing found",
    emptyText: "Reset the filters or choose another category.",
    reset: "Reset filters",
    selectedOnly: "Selected only",
    details: "Open details",
    noEvidence: "Behavior is defined by the Minify configuration",
  },
};

function MinifyPreview({
  item,
  failed,
  onFailure,
}: {
  item: MinifyCatalogItem;
  failed: boolean;
  onFailure: () => void;
}) {
  const source = minifyPreviewUrl(item.preview);
  if (!source || failed) {
    return (
      <div className="minify-preview-fallback">
        <ImageOff />
        <strong>{item.sourceName.slice(0, 2).toUpperCase()}</strong>
      </div>
    );
  }
  return <img src={source} alt="" loading="lazy" decoding="async" onError={onFailure} />;
}

export default function MinifyModsRoute({ language }: { language: Language }) {
  const t = copy[language];
  const [category, setCategory] = useState<MinifyCategory>("all");
  const [query, setQuery] = useState("");
  const [selectedOnly, setSelectedOnly] = useState(false);
  const [activeId, setActiveId] = useState(
    minifyMods.find((item) => item.sourceName === "Misc Optimization")?.id ?? minifyMods[0]?.id ?? "",
  );
  const [failedImages, setFailedImages] = useState<Set<string>>(() => new Set());
  const [selectedIds, setSelectedIds] = useState<string[]>(() =>
    getStoredStringArray("betterfy:selected-minify-mods"));

  const filtered = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase(language);
    return minifyMods.filter((item) => {
      if (category !== "all" && item.category !== category) return false;
      if (selectedOnly && !selectedIds.includes(item.id)) return false;
      if (!needle) return true;
      return [item.name[language], item.sourceName, item.author, item.description[language]]
        .some((value) => value.toLocaleLowerCase(language).includes(needle));
    });
  }, [category, language, query, selectedIds, selectedOnly]);

  const categoryCounts = useMemo(
    () => minifyMods.reduce<Record<MinifyCategory, number>>(
      (counts, item) => {
        counts.all += 1;
        counts[item.category] += 1;
        return counts;
      },
      { all: 0, core: 0, world: 0, interface: 0, audio: 0, custom: 0, utility: 0 },
    ),
    [],
  );

  useEffect(() => {
    if (!filtered.some((item) => item.id === activeId)) setActiveId(filtered[0]?.id ?? "");
  }, [activeId, filtered]);

  const active = filtered.find((item) => item.id === activeId) ?? filtered[0] ?? null;
  const selectedNames = selectedIds
    .map((id) => minifyMods.find((item) => item.id === id)?.name[language])
    .filter(Boolean);

  const toggle = (id: string) => {
    setSelectedIds((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      setStorageItem("betterfy:selected-minify-mods", JSON.stringify(next));
      return next;
    });
  };

  const markFailed = (id: string) =>
    setFailedImages((current) => {
      const next = new Set(current);
      next.add(id);
      return next;
    });

  const evidence = active
    ? [
        [ListX, t.blacklist, active.evidence.blacklistEntries],
        [Files, t.files, active.evidence.includedFiles],
        [Braces, t.styles, active.evidence.stylingRules],
        [FileCode2, t.scripts, active.evidence.scripts],
      ] as const
    : [];

  return (
    <div className="minify-mods-route">
      <header className="minify-heading">
        <div>
          <span><Gauge />{t.eyebrow}</span>
          <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
          <p>{t.text}</p>
        </div>
        <aside className={`minify-selection-summary ${selectedIds.length ? "has-items" : ""}`}>
          <span><Layers3 />{t.selected}</span>
          <strong>{String(selectedIds.length).padStart(2, "0")}</strong>
          <p>{selectedNames.length ? selectedNames.slice(-2).join(" · ") : t.selectedEmpty}</p>
          <small>{t.prototype}</small>
        </aside>
      </header>

      <div className="minify-controls">
        <label>
          <Search />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t.search} />
          {query && <button onClick={() => setQuery("")} aria-label={language === "ru" ? "Очистить поиск" : "Clear search"}>×</button>}
        </label>
        <div role="tablist" aria-label={language === "ru" ? "Категории Minify" : "Minify categories"}>
          {(Object.keys(minifyCategoryLabels) as MinifyCategory[]).map((key) => (
            <button
              role="tab"
              aria-selected={category === key}
              className={category === key ? "active" : ""}
              key={key}
              onClick={() => setCategory(key)}
            >
              <span>{minifyCategoryLabels[key][language]}</span>
              <small>{categoryCounts[key]}</small>
            </button>
          ))}
          <button
            className={`minify-selected-filter ${selectedOnly ? "active" : ""}`}
            aria-pressed={selectedOnly}
            onClick={() => setSelectedOnly((current) => !current)}
            disabled={!selectedIds.length}
          >
            <Check /><span>{t.selectedOnly}</span><small>{selectedIds.length}</small>
          </button>
        </div>
      </div>

      {active ? (
        <>
          <section className="minify-feature" key={active.id}>
            <div className="minify-feature-visual">
              <MinifyPreview item={active} failed={failedImages.has(active.id)} onFailure={() => markFailed(active.id)} />
              <span>{minifyCategoryLabels[active.category][language]}</span>
            </div>
            <div className="minify-feature-copy">
              <span>{t.sourceLabel} · {active.author}</span>
              <h2>{active.name[language]}</h2>
              <p>{active.description[language]}</p>
              <div className="minify-evidence">
                {evidence.map(([Icon, label, value]) => (
                  <div className={value ? "" : "empty"} key={label}>
                    <Icon /><span>{label}</span><strong>{value || "—"}</strong>
                  </div>
                ))}
              </div>
              <button
                className={`minify-primary ${selectedIds.includes(active.id) ? "selected" : ""}`}
                aria-pressed={selectedIds.includes(active.id)}
                onClick={() => toggle(active.id)}
              >
                <span>{selectedIds.includes(active.id) ? <Check /> : <Plus />}</span>
                <strong>{selectedIds.includes(active.id) ? t.remove : t.add}</strong>
                <ArrowRight />
              </button>
              <div className="minify-trust-line">
                <span><ShieldQuestion />{t.compatibility}</span>
                <a href={minifySource.url} target="_blank" rel="noreferrer">{t.source}<ExternalLink /></a>
              </div>
            </div>
          </section>

          <section className="minify-results">
            <header><strong>{filtered.length}</strong><span>{t.results}</span><small>{minifySource.snapshotDate} · {minifySource.commit.slice(0, 8)}</small></header>
            <div className="minify-grid">
              {filtered.map((item) => {
                const selected = selectedIds.includes(item.id);
                const evidenceTotal = Object.values(item.evidence).reduce((sum, value) => sum + value, 0);
                const summary = item.description[language].split(/\n/)[0] || t.noEvidence;
                return (
                  <article className={`${item.id === active.id ? "active" : ""} ${selected ? "selected" : ""}`} key={item.id}>
                    <button
                      className="minify-card-main"
                      aria-label={`${t.details}: ${item.name[language]}`}
                      onClick={() => setActiveId(item.id)}
                    >
                      <div className="minify-card-preview">
                        <MinifyPreview item={item} failed={failedImages.has(item.id)} onFailure={() => markFailed(item.id)} />
                        <span>{minifyCategoryLabels[item.category][language]}</span>
                        {selected && <i><Check />{t.inBuild}</i>}
                      </div>
                      <div className="minify-card-copy">
                        <strong>{item.name[language]}</strong>
                        <p>{summary}</p>
                        <small><span>{item.author}</span><b>{evidenceTotal || "—"}</b></small>
                      </div>
                    </button>
                    <button
                      className="minify-card-toggle"
                      aria-label={`${selected ? t.remove : t.add}: ${item.name[language]}`}
                      aria-pressed={selected}
                      onClick={() => toggle(item.id)}
                    >
                      {selected ? <Check /> : <Plus />}
                      <span>{selected ? t.inBuild : t.add}</span>
                    </button>
                  </article>
                );
              })}
            </div>
          </section>
        </>
      ) : (
        <div className="catalog-empty">
          <Search /><h2>{t.empty}</h2><p>{t.emptyText}</p>
          <button onClick={() => { setQuery(""); setCategory("all"); setSelectedOnly(false); }}>{t.reset}</button>
        </div>
      )}
    </div>
  );
}
