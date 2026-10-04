import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { Check, ChevronDown, Search } from "lucide-react";

// Custom choice controls for the builder. A native <select> renders with the
// system's own popup on Windows, which ignores the theme. Few options become
// radio cards (arrow keys, roving tab stop); many become a listbox popover
// with a search field. Selected means a lime edge and a check mark, so the
// state never depends on colour alone.

export type ChoiceTone = "ready" | "warn" | "accent" | "quiet";

export type ChoiceOption<T extends string> = {
  value: T;
  label: string;
  /** Second line: a state, a native name or a short fact. */
  hint?: string;
  hintIcon?: ReactNode;
  tone?: ChoiceTone;
  badge?: string;
  badgeTone?: ChoiceTone;
  /** Technical detail in monospace, such as the Steam language name. */
  meta?: string;
  /** Extra words for search: native names, engine values. */
  keywords?: string;
  disabled?: boolean;
  /** Takes the whole row in a card grid. */
  wide?: boolean;
};

type Labelled = { label: string; labelledBy?: string };

function OptionText<T extends string>({ option }: { option: ChoiceOption<T> }) {
  return (
    <span className="b-choice-text">
      <b>
        {option.label}
        {option.badge && (
          <em className={`b-choice-badge is-${option.badgeTone ?? "accent"}`}>{option.badge}</em>
        )}
      </b>
      {option.hint && (
        <small className={option.tone ? `is-${option.tone}` : undefined}>
          {option.hintIcon}
          {option.hint}
        </small>
      )}
      {option.meta && <code>{option.meta}</code>}
    </span>
  );
}

const firstEnabled = <T extends string>(options: ChoiceOption<T>[]) =>
  options.findIndex((option) => !option.disabled);

const lastEnabled = <T extends string>(options: ChoiceOption<T>[]) => {
  for (let index = options.length - 1; index >= 0; index -= 1)
    if (!options[index].disabled) return index;
  return -1;
};

// The next enabled option in a direction, wrapping around the ends.
function step<T extends string>(options: ChoiceOption<T>[], from: number, direction: 1 | -1) {
  const count = options.length;
  for (let offset = 1; offset <= count; offset += 1) {
    const index = (((from + direction * offset) % count) + count) % count;
    if (!options[index].disabled) return index;
  }
  return from;
}

export function ChoiceCards<T extends string>({
  label,
  labelledBy,
  value,
  options,
  onChange,
  size = "card",
  disabled = false,
}: Labelled & {
  value: T | "";
  options: ChoiceOption<T>[];
  onChange: (value: T) => void;
  size?: "card" | "chip";
  disabled?: boolean;
}) {
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const checked = options.findIndex((option) => option.value === value && !option.disabled);
  const tabStop = checked >= 0 ? checked : firstEnabled(options);
  const choose = (index: number) => {
    const option = options[index];
    if (!option || option.disabled || disabled || option.value === value) return;
    onChange(option.value);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const next =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? step(options, index, 1)
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? step(options, index, -1)
          : event.key === "Home"
            ? firstEnabled(options)
            : event.key === "End"
              ? lastEnabled(options)
              : -1;
    if (next < 0) return;
    event.preventDefault();
    refs.current[next]?.focus();
    choose(next);
  };
  return (
    <div
      className={`b-choice is-${size}`}
      role="radiogroup"
      aria-label={labelledBy ? undefined : label}
      aria-labelledby={labelledBy}
      aria-disabled={disabled || undefined}
    >
      {options.map((option, index) => {
        const isChecked = option.value === value;
        return (
          <button
            key={option.value}
            ref={(element) => {
              refs.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={isChecked}
            aria-disabled={option.disabled || disabled || undefined}
            tabIndex={index === tabStop ? 0 : -1}
            className={[
              "b-choice-option",
              isChecked ? "is-checked" : "",
              option.wide ? "is-wide" : "",
            ].join(" ")}
            onClick={() => choose(index)}
            onKeyDown={(event) => onKeyDown(event, index)}
          >
            <OptionText option={option} />
            <i className="b-choice-mark" aria-hidden="true">
              {isChecked && <Check />}
            </i>
          </button>
        );
      })}
    </div>
  );
}

// The part of the window a popover can use: the viewport, clipped by every
// scrolling or clipping ancestor (the app scrolls inside .s-scroll, not the
// page).
function visibleBounds(element: HTMLElement) {
  let top = 0;
  let left = 0;
  let bottom = window.innerHeight;
  let right = window.innerWidth;
  for (let node = element.parentElement; node; node = node.parentElement) {
    const style = window.getComputedStyle(node);
    if (/(auto|scroll|hidden|clip)/.test(`${style.overflowX} ${style.overflowY}`)) {
      const rect = node.getBoundingClientRect();
      top = Math.max(top, rect.top);
      left = Math.max(left, rect.left);
      bottom = Math.min(bottom, rect.bottom);
      right = Math.min(right, rect.right);
    }
  }
  return { top, left, bottom, right };
}

const matches = <T extends string>(option: ChoiceOption<T>, needle: string) =>
  `${option.label} ${option.hint ?? ""} ${option.keywords ?? ""}`
    .toLocaleLowerCase()
    .includes(needle);

export function ChoiceMenu<T extends string>({
  label,
  value,
  options,
  onChange,
  placeholder,
  placeholderHint,
  caption,
  searchPlaceholder,
  emptyText,
  disabled = false,
}: Labelled & {
  value: T | "";
  options: ChoiceOption<T>[];
  onChange: (value: T) => void;
  placeholder: string;
  placeholderHint?: string;
  /** One line above the list that applies to every option. */
  caption?: string;
  searchPlaceholder?: string;
  emptyText?: string;
  disabled?: boolean;
}) {
  const id = useId();
  const listId = `${id}list`;
  const captionId = `${id}caption`;
  const optionId = (option: ChoiceOption<T>) => `${id}${option.value}`;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [place, setPlace] = useState({ up: false, right: false, maxHeight: 320, width: 280 });
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const typed = useRef({ text: "", at: 0 });
  const searchable = options.length > 8;
  const selected = options.find((option) => option.value === value);
  const filter = (text: string) => {
    const needle = text.trim().toLocaleLowerCase();
    return needle ? options.filter((option) => matches(option, needle)) : options;
  };
  const shown = filter(query);
  const current = shown[active];

  const openMenu = () => {
    if (disabled) return;
    const index = options.findIndex((option) => option.value === value);
    setQuery("");
    setActive(index >= 0 ? index : Math.max(0, firstEnabled(options)));
    setOpen(true);
  };
  const close = (focusTrigger: boolean) => {
    setOpen(false);
    if (focusTrigger) trigger.current?.focus();
  };
  const pick = (option: ChoiceOption<T> | undefined) => {
    if (!option || option.disabled) return;
    if (option.value !== value) onChange(option.value);
    close(true);
  };

  // Opens below the trigger when there is room, above it otherwise, and
  // never wider or taller than the visible part of the window.
  useLayoutEffect(() => {
    if (!open || !trigger.current) return;
    const rect = trigger.current.getBoundingClientRect();
    const bounds = visibleBounds(trigger.current);
    const below = bounds.bottom - rect.bottom - 10;
    const above = rect.top - bounds.top - 10;
    const up = below < 220 && above > below;
    const width = Math.min(Math.max(rect.width, 260), bounds.right - bounds.left - 16);
    setPlace({
      up,
      right: rect.left + width > bounds.right - 8,
      maxHeight: Math.max(150, Math.min(340, up ? above : below)),
      width,
    });
    (searchable ? search.current : list.current)?.focus({ preventScroll: true });
  }, [open, searchable]);

  // Keep the active option visible inside the list without scrolling the page.
  useEffect(() => {
    const container = list.current;
    const item = current ? document.getElementById(optionId(current)) : null;
    if (!open || !container || !item) return;
    if (item.offsetTop < container.scrollTop) container.scrollTop = item.offsetTop - 6;
    else if (item.offsetTop + item.offsetHeight > container.scrollTop + container.clientHeight)
      container.scrollTop = item.offsetTop + item.offsetHeight - container.clientHeight + 6;
  }, [open, active, query]);

  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [open]);

  const onKey = (event: KeyboardEvent<HTMLElement>) => {
    const move = (to: number) => {
      event.preventDefault();
      if (to >= 0) setActive(to);
    };
    switch (event.key) {
      case "ArrowDown":
        return move(shown.length ? step(shown, active, 1) : -1);
      case "ArrowUp":
        return move(shown.length ? step(shown, active, -1) : -1);
      case "PageDown":
        return move(Math.min(shown.length - 1, active + 8));
      case "PageUp":
        return move(Math.max(0, active - 8));
      case "Enter":
        event.preventDefault();
        return pick(current);
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        return close(true);
      case "Tab":
        return setOpen(false);
    }
    if (searchable) return;
    if (event.key === "Home") return move(firstEnabled(shown));
    if (event.key === "End") return move(lastEnabled(shown));
    if (event.key === " ") {
      event.preventDefault();
      return pick(current);
    }
    // Type-ahead: letters jump to the first option that starts with them.
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      const now = Date.now();
      const text = (now - typed.current.at > 700 ? "" : typed.current.text) + event.key;
      typed.current = { text, at: now };
      const needle = text.toLocaleLowerCase();
      const found = shown.findIndex(
        (option) =>
          !option.disabled &&
          [option.label, option.hint, ...(option.keywords ?? "").split(" ")].some((word) =>
            word?.toLocaleLowerCase().startsWith(needle),
          ),
      );
      if (found >= 0) setActive(found);
    }
  };

  return (
    <div
      ref={root}
      className={`b-choice-menu ${open ? "is-open" : ""} ${selected ? "is-checked" : ""}`}
      onBlur={(event) => {
        if (open && !root.current?.contains(event.relatedTarget as Node | null)) setOpen(false);
      }}
    >
      <button
        ref={trigger}
        type="button"
        className="b-choice-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-label={`${label}: ${selected?.label ?? placeholder}`}
        disabled={disabled}
        onClick={() => (open ? close(false) : openMenu())}
        onKeyDown={(event) => {
          if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
            event.preventDefault();
            openMenu();
          }
        }}
      >
        {selected ? (
          <OptionText option={selected} />
        ) : (
          <span className="b-choice-text">
            <b>{placeholder}</b>
            {placeholderHint && <small>{placeholderHint}</small>}
          </span>
        )}
        {selected && (
          <i className="b-choice-mark" aria-hidden="true">
            <Check />
          </i>
        )}
        <ChevronDown className="b-choice-chevron" aria-hidden="true" />
      </button>
      {open && (
        <div
          className={`b-choice-pop ${place.up ? "is-up" : ""} ${place.right ? "is-right" : ""}`}
          style={{ maxHeight: place.maxHeight, width: place.width }}
        >
          {searchable && (
            <div className="b-choice-search">
              <Search aria-hidden="true" />
              <input
                ref={search}
                type="text"
                role="combobox"
                aria-expanded="true"
                aria-controls={listId}
                aria-autocomplete="list"
                aria-activedescendant={current ? optionId(current) : undefined}
                aria-label={searchPlaceholder ?? label}
                placeholder={searchPlaceholder}
                autoComplete="off"
                spellCheck={false}
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                  setActive(Math.max(0, firstEnabled(filter(event.target.value))));
                }}
                onKeyDown={onKey}
              />
            </div>
          )}
          {caption && (
            <p className="b-choice-caption" id={captionId}>
              {caption}
            </p>
          )}
          <ul
            ref={list}
            id={listId}
            className="b-choice-list"
            role="listbox"
            aria-label={label}
            aria-describedby={caption ? captionId : undefined}
            aria-activedescendant={!searchable && current ? optionId(current) : undefined}
            tabIndex={-1}
            onKeyDown={searchable ? undefined : onKey}
          >
            {shown.map((option, index) => (
              <li
                key={option.value}
                id={optionId(option)}
                role="option"
                aria-selected={option.value === value}
                aria-disabled={option.disabled || undefined}
                className={[
                  index === active ? "is-active" : "",
                  option.value === value ? "is-checked" : "",
                ].join(" ")}
                onMouseDown={(event) => event.preventDefault()}
                onMouseMove={() => {
                  if (index !== active && !option.disabled) setActive(index);
                }}
                onClick={() => pick(option)}
              >
                <OptionText option={option} />
                {option.value === value && <Check className="b-choice-tick" aria-hidden="true" />}
              </li>
            ))}
          </ul>
          {!shown.length && emptyText && (
            <p className="b-choice-empty" role="status">
              {emptyText}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

/** Radio cards for a handful of options, a listbox menu beyond that. */
export function Choice<T extends string>({
  cardLimit = 4,
  placeholder,
  ...props
}: Labelled & {
  value: T | "";
  options: ChoiceOption<T>[];
  onChange: (value: T) => void;
  placeholder: string;
  cardLimit?: number;
  disabled?: boolean;
}) {
  return props.options.length > cardLimit ? (
    <ChoiceMenu {...props} placeholder={placeholder} />
  ) : (
    <ChoiceCards {...props} />
  );
}
