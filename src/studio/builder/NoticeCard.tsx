import { useEffect, useRef } from "react";
import { CircleAlert, TriangleAlert, X } from "lucide-react";
import { useLocale } from "../../i18n";
import type { Notice } from "./notices";

// A failure in the builder: what happened, why, and the one action that
// helps. The engine code sits under "Details"; a developer account sees it
// next to the title.
export function NoticeCard({
  notice,
  actionLabel,
  onAction,
  onDismiss,
  developer = false,
}: {
  notice: Notice;
  actionLabel: string | null;
  onAction?: () => void;
  onDismiss: () => void;
  developer?: boolean;
}) {
  const { isRu } = useLocale();
  const ref = useRef<HTMLDivElement>(null);
  // Bring a new failure into view without stealing keyboard focus.
  useEffect(() => {
    ref.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [notice.code]);
  return (
    <div ref={ref} className={`b-notice is-${notice.tone}`} role="alert" key={notice.code}>
      <span className="b-notice-icon" aria-hidden="true">
        {notice.tone === "error" ? <CircleAlert /> : <TriangleAlert />}
      </span>
      <div className="b-notice-copy">
        <strong>
          {notice.title}
          {developer && <code>{notice.code}</code>}
        </strong>
        <p>{notice.body}</p>
        <div className="b-notice-actions">
          {actionLabel && onAction && (
            <button className="s-btn" type="button" onClick={onAction}>
              {actionLabel}
            </button>
          )}
          {!developer && (
            <details>
              <summary>{isRu ? "Подробности" : "Details"}</summary>
              <code>{notice.code}</code>
            </details>
          )}
        </div>
      </div>
      <button
        className="s-icon b-notice-close"
        type="button"
        onClick={onDismiss}
        aria-label={isRu ? "Скрыть" : "Dismiss"}
      >
        <X />
      </button>
    </div>
  );
}
