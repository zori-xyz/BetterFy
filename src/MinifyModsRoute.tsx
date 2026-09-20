import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRight,
  Braces,
  Check,
  ExternalLink,
  FileCode2,
  Files,
  Gauge,
  ImageOff,
  ListX,
  Plus,
  Search,
  ShieldQuestion,
  X,
} from "lucide-react";
import type { Language } from "./i18n";
import AccentTitle from "./AccentTitle";
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
    eyebrow: "РАБОТА ИГРЫ / BETTERFY",
    title: "Настрой работу Dota",
    text: "Оптимизация, карта, HUD, звук и инструменты. Скины героев и предметов находятся отдельно в Гардеробе.",
    selected: "В СБОРКЕ",
    selectedEmpty: "Выбранные моды BetterFy появятся здесь.",
    prototype: "Выбор сохраняется локально. Установка модов в игру пока не подключена.",
    search: "Найти мод BetterFy",
    add: "Добавить в сборку",
    remove: "Убрать из сборки",
    inBuild: "В сборке",
    source: "Открыть источник",
    sourceLabel: "BETTERFY / ИСХОДНЫЕ ДАННЫЕ",
    unknownPreview: "В исходном репозитории нет превью",
    blacklist: "Удаляемых ресурсов",
    files: "Файлов замены",
    styles: "Строк стилей",
    scripts: "Patch-скриптов",
    compatibility: "Совместимость с текущей Dota не проверена BetterFy",
    results: "модов BetterFy",
    empty: "Ничего не найдено",
    emptyText: "Сбрось фильтры или выбери другую категорию.",
    reset: "Сбросить фильтры",
    selectedOnly: "Только выбранные",
    details: "Открыть детали",
    noEvidence: "Состав определяется конфигурацией мода",
    effect: "ЧТО ИЗМЕНИТСЯ",
    package: "ЧТО ЕСТЬ В ПАКЕТЕ",
    packageEmpty: "В источнике нет числовой разбивки. Поведение задаётся конфигурацией мода.",
  },
  en: {
    eyebrow: "GAME BEHAVIOR / BETTERFY",
    title: "Tune how Dota runs",
    text: "Optimization, map, HUD, audio, and utilities. Hero and item skins live separately in Wardrobe.",
    selected: "IN BUILD",
    selectedEmpty: "Selected BetterFy mods will appear here.",
    prototype: "Selection is stored locally. Installing mods into the game is not connected yet.",
    search: "Find a BetterFy mod",
    add: "Add to build",
    remove: "Remove from build",
    inBuild: "In build",
    source: "Open source",
    sourceLabel: "BETTERFY / SOURCE DATA",
    unknownPreview: "No preview in the source repository",
    blacklist: "Removed resources",
    files: "Replacement files",
    styles: "Style lines",
    scripts: "Patch scripts",
    compatibility: "Compatibility with the current Dota build is not verified by BetterFy",
    results: "BetterFy mods",
    empty: "Nothing found",
    emptyText: "Reset the filters or choose another category.",
    reset: "Reset filters",
    selectedOnly: "Selected only",
    details: "Open details",
    noEvidence: "Behavior is defined by the mod configuration",
    effect: "WHAT CHANGES",
    package: "PACKAGE CONTENT",
    packageEmpty: "The source has no numeric breakdown. Behavior is defined by the mod configuration.",
  },
};

const categoryGuidance: Record<Exclude<MinifyCategory, "all">, Record<Language, string>> = {
  core: {
    ru: "Удаляет или заменяет перечисленные ресурсы, чтобы снизить визуальную нагрузку. BetterFy пока не измерял прирост FPS.",
    en: "Removes or replaces listed resources to reduce visual load. BetterFy has not measured an FPS gain yet.",
  },
  world: {
    ru: "Меняет отображение карты, деревьев, воды или других объектов мира. Проверь требования конкретного мода перед сборкой.",
    en: "Changes the map, trees, water, or other world objects. Review the selected mod requirements before building.",
  },
  interface: {
    ru: "Меняет HUD или меню через файлы и правила интерфейса. Скины героев и предметов этот раздел не содержит.",
    en: "Changes HUD or menus through interface files and rules. This section does not contain hero or item skins.",
  },
  audio: {
    ru: "Отключает или заменяет перечисленные звуки. Изображение служит ориентиром, а точный состав показан в данных пакета.",
    en: "Disables or replaces listed sounds. The image is a reference; the package data describes the actual scope.",
  },
  custom: {
    ru: "Настраивает отдельные элементы клиента Dota. Это функциональная настройка, а не предмет Гардероба.",
    en: "Adjusts individual Dota client elements. This is a functional setting, not a Wardrobe item.",
  },
  utility: {
    ru: "Добавляет служебное действие или автоматизирует конкретную настройку. Внешние patch-скрипты BetterFy не исполняет.",
    en: "Adds a utility action or automates a specific setting. BetterFy does not execute upstream patch scripts.",
  },
};

function MinifyPreview({
  item,
  failed,
  onFailure,
  eager = false,
}: {
  item: MinifyCatalogItem;
  failed: boolean;
  onFailure: () => void;
  eager?: boolean;
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
  return <img src={source} alt="" loading={eager ? "eager" : "lazy"} decoding="async" onError={onFailure} />;
}

export default function MinifyModsRoute({
  language,
  selectedIds,
  onToggle,
}: {
  language: Language;
  selectedIds: string[];
  onToggle: (id: string) => void;
}) {
  const t = copy[language];
  const [category, setCategory] = useState<MinifyCategory>("all");
  const [query, setQuery] = useState("");
  const [selectedOnly, setSelectedOnly] = useState(false);
  const [activeId, setActiveId] = useState(
    minifyMods.find((item) => item.sourceName === "Misc Optimization")?.id ?? minifyMods[0]?.id ?? "",
  );
  const [failedImages, setFailedImages] = useState<Set<string>>(() => new Set());
  const [detailOpen, setDetailOpen] = useState(false);
  const featureRef = useRef<HTMLElement>(null);

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
  const markFailed = (id: string) =>
    setFailedImages((current) => {
      const next = new Set(current);
      next.add(id);
      return next;
    });

  const openItem = (id: string) => {
    setActiveId(id);
    setDetailOpen(true);
    window.requestAnimationFrame(() => featureRef.current?.focus());
  };

  useEffect(() => {
    if (!detailOpen) return undefined;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDetailOpen(false);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [detailOpen]);

  const evidence = active
    ? [
        [ListX, t.blacklist, active.evidence.blacklistEntries],
        [Files, t.files, active.evidence.includedFiles],
        [Braces, t.styles, active.evidence.stylingRules],
        [FileCode2, t.scripts, active.evidence.scripts],
      ] as const
    : [];
  const packageSummary = active
    ? evidence
        .filter(([, , value]) => value > 0)
        .map(([, label, value]) => `${label}: ${value}`)
        .join("; ") || t.packageEmpty
    : "";

  return (
    <div className="minify-mods-route">
      <header className="minify-heading">
        <div>
          <span><Gauge />{t.eyebrow}</span>
          <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
          <p>{t.text}</p>
        </div>
      </header>

      <div className="minify-controls">
        <label>
          <Search />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t.search} />
          {query && <button onClick={() => setQuery("")} aria-label={language === "ru" ? "Очистить поиск" : "Clear search"}>×</button>}
        </label>
        <div className="minify-filter-row">
          <div className="minify-category-scroll" role="tablist" aria-label={language === "ru" ? "Категории модов BetterFy" : "BetterFy mod categories"}>
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
          </div>
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
          {detailOpen && <div className="catalog-detail-layer" onMouseDown={() => setDetailOpen(false)}>
          <section
            ref={featureRef}
            className="minify-feature catalog-detail-dialog"
            key={active.id}
            role="dialog"
            aria-modal="true"
            aria-label={active.name[language]}
            tabIndex={-1}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <button className="catalog-detail-close" aria-label={language === "ru" ? "Закрыть подробности" : "Close details"} onClick={() => setDetailOpen(false)}><X /></button>
            <div className="minify-feature-visual">
              <MinifyPreview item={active} eager failed={failedImages.has(active.id)} onFailure={() => markFailed(active.id)} />
              <span>{minifyCategoryLabels[active.category][language]}</span>
            </div>
            <div className="minify-feature-copy">
              <span>{t.sourceLabel} · {active.author}</span>
              <h2>{active.name[language]}</h2>
              <p>{active.description[language]}</p>
              <div className="minify-context">
                <div><span>{t.effect}</span><p>{categoryGuidance[active.category][language]}</p></div>
                <div><span>{t.package}</span><p>{packageSummary}</p></div>
              </div>
              <div className="minify-evidence">
                {evidence.map(([Icon, label, value]) => (
                  <div className={value ? "" : "empty"} key={label}>
                    <Icon /><span>{label}</span><strong>{value}</strong>
                  </div>
                ))}
              </div>
              <button
                className={`minify-primary ${selectedIds.includes(active.id) ? "selected" : ""}`}
                aria-pressed={selectedIds.includes(active.id)}
                onClick={() => onToggle(active.id)}
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
          </div>}

          <section className="minify-results">
            <header>
              <div><strong>{filtered.length}</strong><span>{t.results}</span></div>
              <small>{language === "ru" ? "Нажми настройку для подробностей или добавь её сразу" : "Open a setting for details or add it immediately"}</small>
            </header>
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
                      onClick={() => openItem(item.id)}
                    >
                      <div className="minify-card-preview">
                        <MinifyPreview item={item} failed={failedImages.has(item.id)} onFailure={() => markFailed(item.id)} />
                        <span>{minifyCategoryLabels[item.category][language]}</span>
                        {selected && <i><Check />{t.inBuild}</i>}
                      </div>
                      <div className="minify-card-copy">
                        <strong>{item.name[language]}</strong>
                        <p>{summary}</p>
                        <small><span>{item.author}</span><b>{evidenceTotal}</b></small>
                        <span className="minify-card-open">{t.details}<ArrowRight /></span>
                      </div>
                    </button>
                    <button
                      className="minify-card-toggle"
                      aria-label={`${selected ? t.remove : t.add}: ${item.name[language]}`}
                      aria-pressed={selected}
                      onClick={() => onToggle(item.id)}
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
