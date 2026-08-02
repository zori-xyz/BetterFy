import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import {
  ArrowRight,
  Check,
  Clipboard,
  Download,
  FileJson,
  Layers3,
  LoaderCircle,
  PackageOpen,
  Plus,
  Save,
  Sparkles,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import {
  presetBridge,
  type BetterFyPreset,
} from "./presets";
import type { Language } from "./i18n";

type ConfigTab = "local" | "workshop";
type TransferState = { mode: "import" | "export"; value: string; preset?: BetterFyPreset } | null;

const copy = {
  ru: {
    eyebrow: "CONFIGS / LOCAL + WORKSHOP",
    title: "Конфиги, которые остаются с тобой",
    text: "Сохраняй состав сборки, переключайся между версиями и передавай пресет одним JSON-файлом.",
    local: "Мои конфиги",
    workshop: "Мастерская BetterFy",
    newName: "Название нового конфига",
    namePlaceholder: "Например, Турбо / чистый HUD",
    saveCurrent: "Сохранить текущий выбор",
    noSelection: "Сначала выбери хотя бы один мод или образ",
    currentCount: "Сейчас выбрано",
    items: "элементов",
    import: "Импортировать JSON",
    emptyTitle: "Первый конфиг ещё не сохранён",
    emptyText: "Собери набор в каталоге и сохрани его здесь — BetterFy не тронет игровые файлы.",
    builtIn: "ПРОВЕРЕННЫЙ СОСТАВ / ЛОКАЛЬНО",
    localLabel: "ЛОКАЛЬНЫЙ ПРОФИЛЬ",
    apply: "Применить",
    applied: "Применён",
    share: "Поделиться",
    remove: "Удалить",
    mods: "модов",
    looks: "образов",
    exportTitle: "Экспорт конфига",
    exportText: "Скопируй JSON и передай его другому пользователю BetterFy.",
    importTitle: "Импорт конфига",
    importText: "Вставь JSON BetterFy. Структура повторно проверится внутри desktop-приложения.",
    copy: "Скопировать",
    copied: "Скопировано",
    importAction: "Проверить и сохранить",
    close: "Закрыть",
    invalid: "Конфиг не прошёл проверку",
    storeError: "Хранилище конфигов недоступно",
    loading: "Читаем локальные конфиги…",
  },
  en: {
    eyebrow: "CONFIGS / LOCAL + WORKSHOP",
    title: "Configurations that stay with you",
    text: "Save a build composition, switch versions, and share a preset as one JSON file.",
    local: "My configs",
    workshop: "BetterFy Workshop",
    newName: "New config name",
    namePlaceholder: "For example, Turbo / clean HUD",
    saveCurrent: "Save current selection",
    noSelection: "Select at least one mod or cosmetic first",
    currentCount: "Currently selected",
    items: "items",
    import: "Import JSON",
    emptyTitle: "No saved configs yet",
    emptyText: "Compose a set in Discover and save it here. BetterFy will not touch game files.",
    builtIn: "CURATED COMPOSITION / LOCAL",
    localLabel: "LOCAL PROFILE",
    apply: "Apply",
    applied: "Applied",
    share: "Share",
    remove: "Delete",
    mods: "mods",
    looks: "cosmetics",
    exportTitle: "Export config",
    exportText: "Copy the JSON and share it with another BetterFy user.",
    importTitle: "Import config",
    importText: "Paste BetterFy JSON. The desktop app validates the structure again.",
    copy: "Copy",
    copied: "Copied",
    importAction: "Validate and save",
    close: "Close",
    invalid: "The config did not pass validation",
    storeError: "Config storage is unavailable",
    loading: "Reading local configs…",
  },
} satisfies Record<Language, Record<string, string>>;

const workshopDescriptions: Record<string, Record<Language, string>> = {
  "betterfy.focus-performance": {
    ru: "Убирает тяжёлые и отвлекающие слои мира, сохраняя читаемую основу игры.",
    en: "Removes heavy, distracting world layers while preserving a readable game foundation.",
  },
  "betterfy.clean-interface": {
    ru: "Спокойный интерфейс: меньше фонового шума, прозрачный HUD и компактная сетка героев.",
    en: "A calmer interface with less background noise, transparent HUD, and a compact hero grid.",
  },
  "betterfy.quiet-match": {
    ru: "Снижает аудионагрузку: окружение, насмешки и голосовые реплики отключаются отдельно.",
    en: "Reduces audio load by separating ambient, taunt, and voice-line controls.",
  },
};

export default function PresetManager({
  language,
  selectedIds,
  onApply,
}: {
  language: Language;
  selectedIds: string[];
  onApply: (modIds: string[]) => void;
}) {
  const t = copy[language];
  const [presets, setPresets] = useState<BetterFyPreset[]>([]);
  const [tab, setTab] = useState<ConfigTab>("local");
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [activeId, setActiveId] = useState<string | null>(null);
  const [transfer, setTransfer] = useState<TransferState>(null);
  const [copied, setCopied] = useState(false);
  const transferRef = useRef<HTMLTextAreaElement>(null);

  const refresh = async () => {
    setLoading(true);
    setError("");
    try {
      setPresets(await presetBridge.list());
    } catch {
      setError(t.storeError);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void refresh(); }, [language]);

  const visible = useMemo(
    () => presets.filter((preset) => tab === "workshop" ? preset.source === "betterfy" : preset.source === "local"),
    [presets, tab],
  );

  const saveCurrent = async () => {
    if (!name.trim() || selectedIds.length === 0) return;
    setSaving(true);
    setError("");
    try {
      const preset = await presetBridge.save({
        name,
        description: language === "ru"
          ? "Локальный конфиг из текущего выбора BetterFy."
          : "Local config created from the current BetterFy selection.",
        modIds: selectedIds,
      });
      setName("");
      setActiveId(preset.id);
      await refresh();
    } catch {
      setError(t.invalid);
    } finally {
      setSaving(false);
    }
  };

  const remove = async (preset: BetterFyPreset) => {
    setError("");
    try {
      await presetBridge.remove(preset.id);
      if (activeId === preset.id) setActiveId(null);
      await refresh();
    } catch {
      setError(t.storeError);
    }
  };

  const openExport = async (preset: BetterFyPreset) => {
    setError("");
    try {
      setTransfer({ mode: "export", preset, value: await presetBridge.export(preset) });
    } catch {
      setError(t.storeError);
    }
  };

  const importPreset = async () => {
    if (!transfer?.value.trim()) return;
    setSaving(true);
    setError("");
    try {
      const preset = await presetBridge.import(transfer.value);
      setTransfer(null);
      setTab("local");
      setActiveId(preset.id);
      await refresh();
    } catch {
      setError(t.invalid);
    } finally {
      setSaving(false);
    }
  };

  const copyExport = async () => {
    if (!transfer) return;
    try {
      await navigator.clipboard.writeText(transfer.value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1400);
    } catch {
      transferRef.current?.focus();
      transferRef.current?.select();
    }
  };

  return (
    <section className="config-manager">
      <header className="config-heading">
        <div>
          <span>{t.eyebrow}</span>
          <h2>{t.title}</h2>
          <p>{t.text}</p>
        </div>
        <div className="config-selection-state">
          <Layers3 />
          <span><small>{t.currentCount}</small><strong>{selectedIds.length} {t.items}</strong></span>
        </div>
      </header>

      <div className="config-toolbar">
        <div className="config-tabs" role="tablist">
          <button className={tab === "local" ? "active" : ""} role="tab" aria-selected={tab === "local"} onClick={() => setTab("local")}>
            <Save />{t.local}
          </button>
          <button className={tab === "workshop" ? "active" : ""} role="tab" aria-selected={tab === "workshop"} onClick={() => setTab("workshop")}>
            <Sparkles />{t.workshop}
          </button>
        </div>
        <button className="config-import-button" onClick={() => setTransfer({ mode: "import", value: "" })}>
          <Upload />{t.import}
        </button>
      </div>

      {tab === "local" && (
        <div className="config-create-row">
          <label>
            <span>{t.newName}</span>
            <input value={name} maxLength={64} placeholder={t.namePlaceholder} onChange={(event) => setName(event.target.value)} />
          </label>
          <button onClick={saveCurrent} disabled={saving || !name.trim() || selectedIds.length === 0} title={selectedIds.length ? t.saveCurrent : t.noSelection}>
            {saving ? <LoaderCircle /> : <Plus />}<strong>{t.saveCurrent}</strong><ArrowRight />
          </button>
        </div>
      )}

      {error && <div className="config-error" role="alert"><FileJson />{error}</div>}

      {loading ? (
        <div className="config-loading"><LoaderCircle />{t.loading}</div>
      ) : visible.length === 0 ? (
        <div className="config-empty">
          <PackageOpen />
          <div><h3>{t.emptyTitle}</h3><p>{t.emptyText}</p></div>
        </div>
      ) : (
        <div className="config-list">
          {visible.map((preset, index) => {
            const applied = activeId === preset.id;
            const description = workshopDescriptions[preset.id]?.[language] ?? preset.description;
            return (
              <article className={`config-card ${applied ? "is-applied" : ""}`} key={preset.id} style={{ "--config-index": index } as CSSProperties}>
                <div className="config-card-mark" aria-hidden="true"><span>{String(index + 1).padStart(2, "0")}</span><i /></div>
                <div className="config-card-copy">
                  <span>{preset.source === "betterfy" ? t.builtIn : t.localLabel}</span>
                  <h3>{preset.name}</h3>
                  <p>{description}</p>
                  <small>{preset.author} · v{preset.version}</small>
                </div>
                <dl>
                  <div><dt>{t.mods}</dt><dd>{preset.modIds.length}</dd></div>
                  <div><dt>{t.looks}</dt><dd>{preset.wardrobeIds.length}</dd></div>
                </dl>
                <div className="config-card-actions">
                  <button className="config-apply" onClick={() => { onApply(preset.modIds); setActiveId(preset.id); }}>
                    {applied ? <Check /> : <Download />}{applied ? t.applied : t.apply}
                  </button>
                  <button onClick={() => void openExport(preset)}><Clipboard /><span>{t.share}</span></button>
                  {!preset.readOnly && <button className="config-delete" onClick={() => void remove(preset)}><Trash2 /><span>{t.remove}</span></button>}
                </div>
              </article>
            );
          })}
        </div>
      )}

      {transfer && (
        <div className="config-transfer-layer" onMouseDown={() => setTransfer(null)}>
          <section role="dialog" aria-modal="true" aria-label={transfer.mode === "export" ? t.exportTitle : t.importTitle} onMouseDown={(event) => event.stopPropagation()}>
            <header>
              <div><span>BETTERFY / JSON</span><h3>{transfer.mode === "export" ? t.exportTitle : t.importTitle}</h3></div>
              <button autoFocus onClick={() => setTransfer(null)} aria-label={t.close}><X /></button>
            </header>
            <p>{transfer.mode === "export" ? t.exportText : t.importText}</p>
            <textarea
              ref={transferRef}
              readOnly={transfer.mode === "export"}
              value={transfer.value}
              spellCheck={false}
              onChange={(event) => setTransfer({ ...transfer, value: event.target.value })}
            />
            <button className="config-transfer-action" onClick={transfer.mode === "export" ? copyExport : importPreset} disabled={saving || !transfer.value.trim()}>
              {saving ? <LoaderCircle /> : transfer.mode === "export" ? <Clipboard /> : <Upload />}
              {transfer.mode === "export" ? (copied ? t.copied : t.copy) : t.importAction}
              <ArrowRight />
            </button>
          </section>
        </div>
      )}
    </section>
  );
}
