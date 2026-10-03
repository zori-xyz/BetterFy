import { useEffect, useId, useRef, useState, type ReactNode, type ImgHTMLAttributes } from "react";
import { ArrowLeft, Check, ImageOff, Plus, X } from "lucide-react";
import { useLocale } from "../i18n";
import { deliveryLabel, isPilotMod, type StudioMod } from "./model";

export function Media({
  src,
  alt = "",
  showPending = false,
  pendingLabel,
  className,
  onLoad,
  ...props
}: ImgHTMLAttributes<HTMLImageElement> & { showPending?: boolean; pendingLabel?: string }) {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const [loadedSrc, setLoadedSrc] = useState<string | null>(null);
  const { isRu } = useLocale();
  if (!src || failedSrc === src)
    return (
      <span className="s-media-fallback">
        <ImageOff />
        <small>{isRu ? "Превью недоступно" : "Preview unavailable"}</small>
      </span>
    );
  const pending = showPending && loadedSrc !== src;
  return (
    <>
      {pending && (
        <span className="s-media-pending" aria-hidden="true">
          <span className="s-media-pending-mark" />
          <small>{pendingLabel ?? (isRu ? "Загружаем превью" : "Loading preview")}</small>
        </span>
      )}
      <img
        {...props}
        className={`${className ?? ""} ${pending ? "s-media-loading" : "s-media-loaded"}`.trim()}
        src={src}
        alt={alt}
        onLoad={(event) => {
          setLoadedSrc(src);
          onLoad?.(event);
        }}
        onError={() => setFailedSrc(src)}
      />
    </>
  );
}

export function Modal({
  title,
  children,
  onClose,
  className = "",
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  className?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const { isRu } = useLocale();
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const dialog = ref.current;
    dialog?.showModal();
    const cancel = (event: Event) => {
      event.preventDefault();
      closeRef.current();
    };
    dialog?.addEventListener("cancel", cancel);
    return () => {
      dialog?.removeEventListener("cancel", cancel);
      dialog?.close();
      previous?.focus();
    };
  }, []);
  return (
    <dialog
      ref={ref}
      className={`s-dialog ${className}`}
      aria-labelledby={titleId}
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          const rect = event.currentTarget.getBoundingClientRect();
          if (
            event.clientX < rect.left ||
            event.clientX > rect.right ||
            event.clientY < rect.top ||
            event.clientY > rect.bottom
          )
            onClose();
        }
      }}
    >
      <header className="s-dialog-head">
        <h2 id={titleId}>{title}</h2>
        <button className="s-icon" aria-label={isRu ? "Закрыть" : "Close"} onClick={onClose}>
          <X />
        </button>
      </header>
      {children}
    </dialog>
  );
}

export function PageHead({
  eyebrow,
  title,
  description,
  children,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  children?: ReactNode;
}) {
  return (
    <header className="s-page-head">
      <div>
        {eyebrow && <span className="s-eyebrow">{eyebrow}</span>}
        <h1>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      {children && <div className="s-head-actions">{children}</div>}
    </header>
  );
}

export function ModCard({
  mod,
  selected,
  favorite,
  onOpen,
  onToggle,
  onFavorite,
}: {
  mod: StudioMod;
  selected: boolean;
  favorite?: boolean;
  onOpen: () => void;
  onToggle: () => void;
  onFavorite?: () => void;
}) {
  const { language, isRu } = useLocale();
  const pilot = isPilotMod(mod.id);
  return (
    <article
      className={`s-mod-card ${selected ? "is-selected" : ""} ${mod.domain === "game" ? "is-game" : ""}`}
      data-spotlight
    >
      <button
        className="s-mod-image"
        onClick={onOpen}
        aria-label={`${isRu ? "Подробнее" : "Details"}: ${mod.name[language]}`}
      >
        <Media
          src={mod.image ?? undefined}
          loading="lazy"
          showPending
          pendingLabel={mod.categoryName[language]}
        />
        {/* Preview is the default and is explained once above the grid; only
            the installable pilot mods carry a badge. */}
        {pilot && (
          <span
            className="s-delivery-flag is-pilot"
            title={
              isRu
                ? "Доступно для проверяемой установки в Windows-пилоте"
                : "Available for verifiable installation in the Windows pilot"
            }
          >
            {deliveryLabel("pilot", language)}
          </span>
        )}
        {selected && (
          <span className="s-selected-flag">
            <Check />
            {isRu ? "В сборке" : "In build"}
          </span>
        )}
      </button>
      <div className="s-mod-copy">
        <span className="s-mod-category">{mod.categoryName[language]}</span>
        <button className="s-mod-name" onClick={onOpen}>
          {mod.name[language]}
        </button>
        {mod.domain === "game" && <p>{mod.description[language].split("\n")[0]}</p>}
        <div className="s-mod-bottom">
          <span title={mod.author}>{mod.author}</span>
          <button
            className={`s-add ${selected ? "is-selected" : ""}`}
            onClick={onToggle}
            aria-label={`${selected ? (isRu ? "Убрать" : "Remove") : isRu ? "Добавить" : "Add"}: ${mod.name[language]}`}
            aria-pressed={selected}
          >
            {selected ? <Check /> : <Plus />}
          </button>
        </div>
      </div>
    </article>
  );
}

export function Empty({
  icon,
  title,
  text,
  action,
  onAction,
}: {
  icon: ReactNode;
  title: string;
  text: string;
  action?: string;
  onAction?: () => void;
}) {
  return (
    <div className="s-empty">
      <span>{icon}</span>
      <h2>{title}</h2>
      <p>{text}</p>
      {action && (
        <button className="s-btn s-btn-primary" onClick={onAction}>
          {action}
          <ArrowLeft className="s-arrow-forward" />
        </button>
      )}
    </div>
  );
}
