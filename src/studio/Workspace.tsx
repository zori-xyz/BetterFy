import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, ArrowUpRight, Check, ChevronDown, ChevronRight, CircleUserRound, Compass, FolderHeart, Gamepad2, Layers3, LoaderCircle, Plus, Search, Send, Settings, SlidersHorizontal, Sparkles, TriangleAlert, X } from "lucide-react";
import AuthFlow from "../AuthFlow";
import OnboardingFlow from "../OnboardingFlow";
import AppUpdater from "../AppUpdater";
import BetterFyWordmark from "../BetterFyWordmark";
import { fetchTelegramAvatar, restoreDesktopSession, revokeAuthSession, type AuthSession } from "../auth";
import type { GameInstallation } from "../engine";
import { useLocale, modCount } from "../i18n";
import { getStorageItem, getStoredStringArray, setStorageItem } from "../storage";
import { presetBridge, type BetterFyPreset } from "../presets";
import Catalog, { initialFilters, ModDetails, type CatalogFilters } from "./Catalog";
import Home from "./Home";
import Build, { initialBuild, type BuildState } from "./Build";
import Library from "./Library";
import { Preferences, Profile } from "./Preferences";
import { modById, type Domain, type StudioMod } from "./model";
import { Media, Modal } from "./ui";

type Route = "home" | "catalog" | "build" | "library" | "settings" | "profile";
type Stage = "loading" | "auth" | "setup" | "workspace";
type Theme = "dark" | "light";
const installationKey = "betterfy:game-installation";
const previewSession: AuthSession = { userId: "betterfy-preview", displayName: "Preview", accessTier: "preview", source: "demo" };
const previewInstallation: GameInstallation = { path: "Preview / Dota 2", executablePath: "Preview only", steamLibrary: "BetterFy interface preview", client: "Prototype preview", source: "demo", verified: false };
function storedInstallation(): GameInstallation | null { try { const item = JSON.parse(getStorageItem(installationKey) ?? "null"); return item && typeof item.path === "string" && typeof item.verified === "boolean" ? item : null; } catch { return null; } }
function storedVariants(): Record<string, number> { try { const value = JSON.parse(getStorageItem("betterfy:studio-variants") ?? "{}"); return Object.fromEntries(Object.entries(value).filter(([, index]) => typeof index === "number" && Number.isInteger(index) && index >= 0)) as Record<string, number>; } catch { return {}; } }

export default function Workspace({ session, installation, theme, setTheme, motion, setMotion, onReconnect, onSignOut }: { session: AuthSession | null; installation: GameInstallation; theme: Theme; setTheme: (theme: Theme) => void; motion: boolean; setMotion: (motion: boolean) => void; onReconnect: () => void; onSignOut: () => void }) {
  const { isRu, language } = useLocale();
  const [route, setRoute] = useState<Route>("home");
  const [selected, setSelected] = useState<string[]>(() => [...new Set([...getStoredStringArray("betterfy:selected-mods"), ...getStoredStringArray("betterfy:selected-minify-mods")])]);
  const [favorites, setFavorites] = useState<string[]>(() => getStoredStringArray("betterfy:studio-favorites"));
  const [variants, setVariants] = useState<Record<string, number>>(storedVariants);
  const [filters, setFilters] = useState<CatalogFilters>(initialFilters);
  const [details, setDetails] = useState<StudioMod | null>(null);
  const [replacement, setReplacement] = useState<{ mod: StudioMod; others: StudioMod[] } | null>(null);
  const [collection, setCollection] = useState<{ preset: BetterFyPreset; replace: boolean } | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [saveBusy, setSaveBusy] = useState(false);
  const [saveError, setSaveError] = useState(false);
  const [libraryRevision, setLibraryRevision] = useState(0);
  const [build, setBuild] = useState<BuildState>(initialBuild);
  const [toast, setToast] = useState<{ text: string; key: number } | null>(null);
  const [launchNotice, setLaunchNotice] = useState(false);
  const [storageError, setStorageError] = useState(false);
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollPositions = useRef<Partial<Record<Route, number>>>({});
  const currentRoute = useRef(route);
  const buildTimer = useRef<number | null>(null);
  const busy = build.phase === "preparing" || build.phase === "restoring";
  const preview = !session || session.source === "demo";
  useEffect(() => {
    let active = true;
    let objectUrl: string | null = null;
    setAvatarUrl(null);
    if (preview || !session?.avatarAvailable) return undefined;
    fetchTelegramAvatar(session).then(blob => {
      if (!active || !blob) return;
      objectUrl = URL.createObjectURL(blob);
      setAvatarUrl(objectUrl);
    }).catch(() => undefined);
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [session, preview]);
  const labels = { home: isRu ? "Обзор" : "Overview", catalog: isRu ? "Каталог" : "Discover", build: isRu ? "Моя сборка" : "My build", library: isRu ? "Библиотека" : "Library", settings: isRu ? "Настройки" : "Settings", profile: isRu ? "Профиль" : "Profile" };
  const notify = (text: string) => setToast({ text, key: Date.now() });
  useEffect(() => { if (!toast) return; const timer = window.setTimeout(() => setToast(null), 3800); return () => window.clearTimeout(timer); }, [toast]);
  useEffect(() => { setToast(null); }, [language]);
  useEffect(() => () => { if (buildTimer.current !== null) window.clearInterval(buildTimer.current); }, []);
  useLayoutEffect(() => { if (scrollRef.current) scrollRef.current.scrollTop = scrollPositions.current[route] ?? 0; currentRoute.current = route; }, [route]);
  const navigate = (next: Route) => { if (next === route) return; if (scrollRef.current) scrollPositions.current[route] = scrollRef.current.scrollTop; setToast(null); setRoute(next); };
  const openCatalog = (domain?: Domain) => { if (domain) setFilters(previous => ({ ...initialFilters, domain })); navigate("catalog"); };
  const search = () => { navigate("catalog"); window.requestAnimationFrame(() => document.getElementById("studio-catalog-search")?.focus()); };
  useEffect(() => { const handler = (event: KeyboardEvent) => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); if (!document.querySelector("dialog[open]")) { if (scrollRef.current) scrollPositions.current[currentRoute.current] = scrollRef.current.scrollTop; setRoute("catalog"); window.requestAnimationFrame(() => document.getElementById("studio-catalog-search")?.focus()); } } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, []);
  const saveSelection = (ids: string[]) => {
    setSelected(ids); setBuild(initialBuild);
    const game = ids.filter(id => id.startsWith("minify-"));
    const wardrobe = ids.filter(id => !id.startsWith("minify-"));
    const first = setStorageItem("betterfy:selected-minify-mods", JSON.stringify(game));
    const second = setStorageItem("betterfy:selected-mods", JSON.stringify(wardrobe));
    setStorageError(!first || !second);
  };
  const toggle = (mod: StudioMod) => {
    if (busy) { notify(isRu ? "Дождись завершения или отмени подготовку" : "Wait for preparation to finish or cancel it"); return; }
    if (selected.includes(mod.id)) { saveSelection(selected.filter(id => id !== mod.id)); notify(isRu ? "Мод убран из сборки" : "Mod removed from build"); return; }
    const others = mod.slot ? selected.flatMap(id => { const other = modById.get(id); return other?.slot === mod.slot ? [other] : []; }) : [];
    if (others.length) { setDetails(null); setReplacement({ mod, others }); return; }
    saveSelection([...selected, mod.id]); notify(isRu ? "Добавлено в твою сборку" : "Added to your build");
  };
  const saveVariant = (mod: StudioMod, value: number) => { if (busy) return; const next = { ...variants, [mod.id]: value }; setVariants(next); if (!setStorageItem("betterfy:studio-variants", JSON.stringify(next))) setStorageError(true); if (selected.includes(mod.id)) setBuild(initialBuild); };
  const toggleFavorite = (mod: StudioMod) => { const next = favorites.includes(mod.id) ? favorites.filter(id => id !== mod.id) : [...favorites, mod.id]; setFavorites(next); if (!setStorageItem("betterfy:studio-favorites", JSON.stringify(next))) setStorageError(true); };
  const openSave = () => { if (!selected.length) { notify(isRu ? "Сначала добавь хотя бы один мод" : "Add at least one mod first"); return; } setSaveName(isRu ? "Моя Dota" : "My Dota"); setSaveError(false); setSaving(true); };
  const savePreset = async () => { setSaveBusy(true); setSaveError(false); try { await presetBridge.save({ name: saveName, description: "", modIds: selected.filter(id => id.startsWith("minify-")), wardrobeIds: selected.filter(id => !id.startsWith("minify-")) }); setSaving(false); setLibraryRevision(value => value + 1); notify(isRu ? "Набор сохранён в библиотеке" : "Build saved to your library"); } catch { setSaveError(true); } finally { setSaveBusy(false); } };
  const startBuild = () => { if (busy || !selected.length) return; const startedAt = Date.now(); setBuild({ phase: "preparing", progress: 0, completedIds: [], startedAt }); buildTimer.current = window.setInterval(() => { const progress = Math.min(100, Math.floor((Date.now() - startedAt) / 65)); if (progress >= 100) { if (buildTimer.current !== null) window.clearInterval(buildTimer.current); buildTimer.current = null; setBuild({ phase: "ready", progress: 100, completedIds: [...selected], startedAt }); notify(isRu ? "Предпросмотр сборки завершён" : "Build preview complete"); } else setBuild(previous => ({ ...previous, progress })); }, 100); };
  const cancelBuild = () => { if (buildTimer.current !== null) window.clearInterval(buildTimer.current); buildTimer.current = null; setBuild(initialBuild); notify(isRu ? "Подготовка отменена. Твой выбор сохранён." : "Preparation cancelled. Your selection is saved."); };
  return <main className="s-app">
    <a className="s-skip" href="#studio-content">{isRu ? "К содержимому" : "Skip to content"}</a>
    <aside className="s-sidebar"><div className="s-brand" data-tauri-drag-region><BetterFyWordmark /></div><button className="s-game-switch" onClick={onReconnect} disabled={busy}><span className="s-dota-icon"><Gamepad2 /></span><span><strong>Dota 2</strong><small>{isRu ? "Твоя игра" : "Your game"}</small></span><ChevronDown /></button><span className="s-nav-label">{isRu ? "ПРОСТРАНСТВО" : "WORKSPACE"}</span><nav aria-label={isRu ? "Основная навигация" : "Main navigation"}>{([{ id: "home", icon: Compass }, { id: "catalog", icon: SlidersHorizontal }, { id: "build", icon: Layers3 }, { id: "library", icon: FolderHeart }] as const).map(({ id, icon: Icon }) => <button key={id} aria-current={route === id ? "page" : undefined} onClick={() => navigate(id)}><Icon /><span>{labels[id]}</span>{id === "build" && selected.length > 0 && <b>{selected.length}</b>}</button>)}</nav><div className="s-sidebar-bottom">{selected.length > 0 && <button className="s-sidebar-build" onClick={() => navigate("build")}><span>{busy ? <LoaderCircle className="s-spin" /> : <Layers3 />}<strong>{busy ? `${build.progress}%` : modCount(selected.length, language)}</strong><ChevronRight /></span><small>{busy ? (isRu ? "Готовим твою сборку" : "Preparing your build") : (isRu ? "Продолжить настройку" : "Continue your build")}</small>{busy && <i style={{ width: `${build.progress}%` }} />}</button>}<nav><button aria-current={route === "settings" ? "page" : undefined} onClick={() => navigate("settings")}><Settings /><span>{labels.settings}</span></button><a href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer"><Send /><span>{isRu ? "Помощь и идеи" : "Help & ideas"}</span><ArrowUpRight /></a><a href="https://zori-xyz.github.io/BetterFy/" target="_blank" rel="noreferrer"><Sparkles /><span>{isRu ? "Сайт BetterFy" : "BetterFy website"}</span><ArrowUpRight /></a></nav><button className="s-sidebar-profile" onClick={() => navigate("profile")} aria-current={route === "profile" ? "page" : undefined}><span className="s-mini-avatar">{avatarUrl ? <img src={avatarUrl} alt="" /> : <CircleUserRound />}</span><span><strong>{preview ? (isRu ? "Гость BetterFy" : "BetterFy guest") : session.displayName}</strong><small>{preview ? (isRu ? "Знакомство с приложением" : "Exploring the app") : (isRu ? "Аккаунт BetterFy" : "BetterFy account")}</small></span><ChevronRight /></button></div></aside>
    <section className="s-workspace"><header className="s-topbar" data-tauri-drag-region><div className="s-breadcrumb"><span>Dota 2</span><ChevronRight /><strong>{labels[route]}</strong></div><button className="s-top-search" onClick={search}><Search /><span>{isRu ? "Найти мод" : "Find a mod"}</span><kbd>{/Mac|iPhone|iPad/.test(navigator.platform) ? "⌘ K" : "Ctrl K"}</kbd></button><button className="s-prototype-badge" onClick={() => navigate("settings")} title={selected.includes("minify-tree-mod") ? (isRu ? "Реальная установка доступна только для Tree Mod на Windows." : "Real installation is available only for Tree Mod on Windows.") : (isRu ? "Интерактивный прототип. Файлы Dota 2 не изменяются." : "Interactive prototype. Dota 2 files are not changed.")}>{selected.includes("minify-tree-mod") ? (isRu ? "Пилот Tree Mod" : "Tree Mod pilot") : (isRu ? "Прототип" : "Prototype")}</button></header>
      {storageError && <div className="s-storage-warning" role="alert"><TriangleAlert />{isRu ? "Не удалось сохранить изменения на устройстве. Оставь приложение открытым." : "Changes could not be saved on this device. Keep the app open."}</div>}
      <div className="s-scroll" id="studio-content" ref={scrollRef} tabIndex={-1}><div className={`s-page s-page-${route}`} key={route}>
        {route === "home" && <Home selected={selected} motion={motion} onCatalog={openCatalog} onBuild={() => navigate("build")} onOpen={setDetails} onToggle={toggle} onCollection={preset => setCollection({ preset, replace: false })} />}
        {route === "catalog" && <Catalog filters={filters} setFilters={setFilters} selected={selected} favorites={favorites} onOpen={setDetails} onToggle={toggle} />}
        {route === "build" && <Build ids={selected} variants={variants} state={build} installation={installation} preview={preview} onRemove={toggle} onOpen={setDetails} onCatalog={() => openCatalog()} onStart={startBuild} onCancel={cancelBuild} onSave={openSave} onReset={() => setBuild(initialBuild)} onResolve={(keep, alternatives) => saveSelection(selected.filter(id => id === keep.id || !alternatives.some(mod => mod.id === id)))} onPlay={() => setLaunchNotice(true)} />}
        {route === "library" && <Library onApply={preset => setCollection({ preset, replace: true })} onSave={openSave} onCatalog={() => openCatalog()} notify={notify} revision={libraryRevision} />}
        {route === "settings" && <Preferences theme={theme} setTheme={setTheme} motion={motion} setMotion={setMotion} installation={installation} onReconnect={onReconnect} />}
        {route === "profile" && <Profile session={session} avatarUrl={avatarUrl} onSignOut={onSignOut} />}
        <footer className="s-page-footer"><span>BetterFy · Dota 2</span><span>{isRu ? "Твои моды. Твой выбор." : "Your mods. Your choice."}</span></footer>
      </div></div>
    </section>
    <div className="s-toast-region" role="status" aria-live="polite">{toast && <div className="s-toast" key={toast.key}><Check /><span>{toast.text}</span><button className="s-icon" onClick={() => setToast(null)} aria-label={isRu ? "Закрыть уведомление" : "Dismiss notification"}><X /></button></div>}</div>
    {details && <ModDetails key={details.id} mod={details} selected={selected.includes(details.id)} favorite={favorites.includes(details.id)} variant={variants[details.id] ?? 0} onVariant={value => saveVariant(details, value)} onClose={() => setDetails(null)} onToggle={() => toggle(details)} onFavorite={() => toggleFavorite(details)} />}
    {replacement && <Modal title={isRu ? "Заменить выбранный мод?" : "Replace the selected mod?"} onClose={() => setReplacement(null)}><div className="s-dialog-body"><p>{isRu ? "Эти варианты занимают один слот. В сборке может остаться один из них." : "These options share a slot. Keep one of them in your build."}</p><div className="s-replacement"><div><Media src={replacement.others[0].image ?? undefined} /><small>{isRu ? "Сейчас" : "Current"}</small><strong>{replacement.others[0].name[language]}</strong></div><ArrowRight /><div><Media src={replacement.mod.image ?? undefined} /><small>{isRu ? "Новый вариант" : "New option"}</small><strong>{replacement.mod.name[language]}</strong></div></div><div className="s-dialog-actions"><button className="s-btn" onClick={() => setReplacement(null)}>{isRu ? "Оставить текущий" : "Keep current"}</button><button className="s-btn s-btn-primary" onClick={() => { saveSelection([...selected.filter(id => !replacement.others.some(mod => mod.id === id)), replacement.mod.id]); setReplacement(null); notify(isRu ? "Мод заменён в сборке" : "Mod replaced in your build"); }}>{isRu ? "Заменить" : "Replace"}<ArrowRight /></button></div></div></Modal>}
    {collection && <Modal title={collection.preset.name} onClose={() => setCollection(null)}><div className="s-dialog-body"><p>{collection.replace ? (isRu ? "Этот набор заменит текущий выбор. Сохранённые наборы останутся в библиотеке." : "This build replaces your current selection. Saved builds stay in your library.") : (isRu ? "Добавь весь набор или сначала посмотри, что в него входит." : "Add the collection or take a look at what's inside first.")}</p><div className="s-collection-detail">{[...collection.preset.modIds, ...collection.preset.wardrobeIds].map(id => { const mod = modById.get(id); return <div key={id}><span>{mod ? <Media src={mod.image ?? undefined} /> : <TriangleAlert />}</span><strong>{mod?.name[language] ?? id}</strong>{selected.includes(id) && <Check />}</div>; })}</div><button className="s-btn s-btn-primary s-btn-full" disabled={busy} onClick={() => { const ids = [...collection.preset.modIds, ...collection.preset.wardrobeIds]; saveSelection(collection.replace ? ids : [...new Set([...selected, ...ids])]); setCollection(null); navigate("build"); notify(isRu ? "Набор открыт в сборке" : "Collection opened in your build"); }}>{collection.replace ? (isRu ? "Использовать этот набор" : "Use this build") : (isRu ? "Добавить весь набор" : "Add collection")}<ArrowRight /></button></div></Modal>}
    {saving && <Modal title={isRu ? "Сохранить набор" : "Save build"} onClose={() => { if (!saveBusy) setSaving(false); }}><form className="s-dialog-body" onSubmit={event => { event.preventDefault(); void savePreset(); }}><p>{isRu ? "Дай набору имя, чтобы легко вернуться к нему." : "Give your build a name so you can find it again."}</p><label className="s-field"><span>{isRu ? "Название" : "Name"}</span><input autoFocus value={saveName} maxLength={64} onChange={event => setSaveName(event.target.value)} /></label><div className="s-save-count"><Layers3 />{modCount(selected.length, language)}</div>{saveError && <p className="s-form-error" role="alert">{isRu ? "Не удалось сохранить набор. Проверь доступ к хранилищу." : "Could not save the build. Check storage access."}</p>}<button className="s-btn s-btn-primary s-btn-full" disabled={!saveName.trim() || saveBusy}>{saveBusy ? <LoaderCircle className="s-spin" /> : <Check />}{isRu ? "Сохранить в библиотеку" : "Save to library"}</button></form></Modal>}
    {launchNotice && <Modal title={isRu ? "Твой набор готов к игре" : "Your build is ready to play"} onClose={() => setLaunchNotice(false)}><div className="s-dialog-body"><div className="s-launch-game"><Gamepad2 /><strong>Dota 2</strong></div><p>{isRu ? "Ты прошёл полный сценарий новой оболочки. Запуск игры и установка модов подключаются следующим этапом; сейчас файлы Dota не изменены." : "You've completed the new interface flow. Game launch and mod installation will be connected next; Dota files have not been changed."}</p><button className="s-btn s-btn-primary s-btn-full" onClick={() => { setLaunchNotice(false); navigate("home"); }}>{isRu ? "Вернуться на главную" : "Back to overview"}<ArrowRight /></button></div></Modal>}
  </main>;
}
