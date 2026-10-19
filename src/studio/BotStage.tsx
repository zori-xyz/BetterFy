import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  ArrowUpRight,
  Check,
  Github,
  Globe2,
  MessageCircleHeart,
  Send,
  ShieldCheck,
  Star,
  UserRound,
} from "lucide-react";
import { useLocale } from "../i18n";
import { emoteMoods } from "./EmoteStage";
import { useMotion } from "./useMotion";
import botAvatar from "../../website/public/betterfy-logo.png";

// The end of Home: BetterFy's Telegram, told by the bot itself. A chat window
// plays scenes of real bot messages (services/auth-worker/src/copy.mjs, word
// for word) while the section is on screen: sign-in, Premium, support and the
// account. Each point on the left is one scene; the playing one lights up and
// any can be picked. Emoticon stickers float around the chat in the page's
// mood colour. Reduced motion shows the whole scene at once.

type Pair = [ru: string, en: string];
type Line =
  | { from: "bot"; text: Pair; keys?: Pair[]; pressed?: number }
  | { from: "me"; text: Pair };
type Scene = { id: string; icon: ReactNode; point: Pair; lines: Line[] };

const scenes: Scene[] = [
  {
    id: "signin",
    icon: <ShieldCheck />,
    point: [
      "Вход без паролей: подтверждаешь одной кнопкой в боте.",
      "No passwords: approve the sign-in with one tap in the bot.",
    ],
    lines: [
      {
        from: "bot",
        text: [
          "BetterFy запрашивает вход на новом устройстве. Подтверди его только если ты сам открыл приложение.",
          "BetterFy is requesting a sign-in on a new device. Approve it only if you opened the app yourself.",
        ],
        keys: [
          ["Подтвердить вход", "Approve sign-in"],
          ["Отклонить", "Deny"],
        ],
        pressed: 0,
      },
      {
        from: "bot",
        text: [
          "Вход подтверждён. Вернись в BetterFy — приложение завершит авторизацию само.",
          "Sign-in approved. Return to BetterFy — the app will finish automatically.",
        ],
      },
    ],
  },
  {
    id: "premium",
    icon: <Star />,
    point: [
      "Premium за Telegram Stars, от трёх дней.",
      "Premium with Telegram Stars, from three days.",
    ],
    lines: [
      { from: "me", text: ["/subscribe", "/subscribe"] },
      {
        from: "bot",
        text: [
          "Выбери срок доступа. 3 и 15 дней оплачиваются один раз. План на 30 дней продлевается автоматически — его можно отключить в любой момент.",
          "Choose an access period. The 3-day and 15-day passes are one-time purchases. The 30-day plan renews automatically and can be stopped at any time.",
        ],
        keys: [
          ["3 дня · 75 ⭐", "3 days · 75 ⭐"],
          ["15 дней · 225 ⭐", "15 days · 225 ⭐"],
          ["30 дней · 525 ⭐", "30 days · 525 ⭐"],
        ],
        pressed: 1,
      },
      {
        from: "bot",
        text: [
          "Оплата прошла. Premium уже привязан к твоему профилю BetterFy.",
          "Payment received. Premium is now linked to your BetterFy profile.",
        ],
      },
    ],
  },
  {
    id: "support",
    icon: <MessageCircleHeart />,
    point: [
      "В @BeterHelp отвечают люди, которые делают BetterFy.",
      "At @BeterHelp the people who build BetterFy answer.",
    ],
    lines: [
      { from: "me", text: ["/support", "/support"] },
      {
        from: "bot",
        text: [
          "С любым вопросом, ошибкой или оплатой пиши в @BeterHelp. Приложи, что случилось и когда. Не присылай коды входа, токены и данные кошелька.",
          "For any question, bug or payment, message @BeterHelp. Say what happened and when. Never send sign-in codes, tokens or wallet details.",
        ],
        keys: [["Поддержка", "Support"]],
      },
    ],
  },
  {
    id: "account",
    icon: <UserRound />,
    point: [
      "Все входы видны в боте, выйти везде можно одной кнопкой.",
      "Every sign-in is listed in the bot; sign out everywhere with one tap.",
    ],
    lines: [
      { from: "me", text: ["/status", "/status"] },
      {
        from: "bot",
        text: [
          "Твой аккаунт BetterFy\nДоступ: ранний доступ\nАктивных входов: 2",
          "Your BetterFy account\nAccess: Early Access\nActive sign-ins: 2",
        ],
        keys: [
          ["Мои входы", "My sign-ins"],
          ["Выйти везде", "Sign out everywhere"],
        ],
      },
    ],
  },
];

const STEP = 1600;
const HOLD = 3600;

function useScenes(running: boolean) {
  const [scene, setScene] = useState(0);
  const [shown, setShown] = useState(scenes[0].lines.length);
  useEffect(() => {
    if (running) setShown(0);
    else setShown(scenes[scene].lines.length);
    // Only starting or pausing resets what is shown.
  }, [running]);
  useEffect(() => {
    if (!running) return undefined;
    const lines = scenes[scene].lines;
    const done = shown >= lines.length;
    const timer = window.setTimeout(
      () => {
        if (!done) setShown(shown + 1);
        else {
          setScene((scene + 1) % scenes.length);
          setShown(0);
        }
      },
      done ? HOLD : STEP,
    );
    return () => window.clearTimeout(timer);
  }, [running, scene, shown]);
  const pick = (index: number) => {
    setScene(index);
    setShown(running ? 0 : scenes[index].lines.length);
  };
  const next = scenes[scene].lines[shown];
  return { scene, shown, typing: running && next?.from === "bot", pick };
}

export function BotStage({ motion }: { motion: boolean }) {
  const { isRu } = useLocale();
  const t = (ru: string, en: string) => (isRu ? ru : en);
  const { animated } = useMotion(motion);
  const stage = useRef<HTMLDivElement>(null);
  const [inView, setInView] = useState(false);
  useEffect(() => {
    const element = stage.current;
    if (!element) return undefined;
    const observer = new IntersectionObserver(([entry]) => setInView(entry.isIntersecting), {
      threshold: 0.35,
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const running = animated && inView;
  const { scene, shown, typing, pick } = useScenes(running);
  const current = scenes[scene];
  const stickers = [emoteMoods[1], emoteMoods[3], emoteMoods[8]];

  return (
    <section className="h-bot" aria-labelledby="h-bot-title">
      <div className="h-bot-copy">
        <span className="s-eyebrow">
          <Send />
          {t("BETTERFY В TELEGRAM", "BETTERFY ON TELEGRAM")}
        </span>
        <h2 id="h-bot-title">
          {t("Вход, Premium и поддержка — ", "Sign-in, Premium and support — ")}
          <span>{t("в одном чате", "in one chat")}</span>
        </h2>
        <ul>
          {scenes.map((item, index) => (
            <li key={item.id}>
              <button
                type="button"
                className={index === scene ? "is-active" : ""}
                aria-pressed={index === scene}
                onClick={() => pick(index)}
              >
                {item.icon}
                <span>{item.point[isRu ? 0 : 1]}</span>
                {index === scene && running && (
                  <i
                    key={`${scene}-${shown === 0}`}
                    className="h-bot-progress"
                    style={{
                      animationDuration: `${item.lines.length * STEP + HOLD}ms`,
                    }}
                  />
                )}
              </button>
            </li>
          ))}
        </ul>
        <div className="h-bot-actions">
          <a
            className="s-btn s-btn-primary"
            href="https://t.me/BeterFyBot"
            target="_blank"
            rel="noreferrer"
          >
            {t("Открыть @BeterFyBot", "Open @BeterFyBot")}
            <ArrowUpRight />
          </a>
          <a
            className="s-btn s-btn-ghost"
            href="https://t.me/BeterHelp"
            target="_blank"
            rel="noreferrer"
          >
            {t("Написать в поддержку", "Message support")}
            <ArrowUpRight />
          </a>
        </div>
        <nav className="h-bot-links" aria-label={t("Ещё о BetterFy", "More BetterFy")}>
          <a href="https://zori-xyz.github.io/BetterFy/" target="_blank" rel="noreferrer">
            <Globe2 />
            {t("Сайт BetterFy", "BetterFy website")}
          </a>
          <a href="https://github.com/zori-xyz/BetterFy/releases" target="_blank" rel="noreferrer">
            <Github />
            {t("Релизы на GitHub", "Releases on GitHub")}
          </a>
        </nav>
      </div>

      <div
        className="h-bot-stage"
        ref={stage}
        aria-hidden="true"
        onPointerMove={(event) => {
          if (!animated || event.pointerType !== "mouse") return;
          const rect = event.currentTarget.getBoundingClientRect();
          const x = (event.clientX - rect.left) / rect.width - 0.5;
          const y = (event.clientY - rect.top) / rect.height - 0.5;
          event.currentTarget.style.setProperty("--px", x.toFixed(3));
          event.currentTarget.style.setProperty("--py", y.toFixed(3));
        }}
        onPointerLeave={(event) => {
          event.currentTarget.style.setProperty("--px", "0");
          event.currentTarget.style.setProperty("--py", "0");
        }}
      >
        {stickers.map((mood, index) => (
          <img key={mood.name} className={`h-bot-sticker is-${index}`} src={mood.still} alt="" />
        ))}
        <div className="h-bot-chat">
          <header>
            <img src={botAvatar} alt="" />
            <span>
              <strong>BetterFy</strong>
              <small>{typing ? t("печатает…", "typing…") : t("бот", "bot")}</small>
            </span>
          </header>
          <ol key={current.id}>
            {current.lines.slice(0, shown).map((line, index) => (
              <li key={index} className={`h-bot-line is-${line.from}`}>
                <p>{line.text[isRu ? 0 : 1]}</p>
                {line.from === "bot" && line.keys && (
                  <span className={`h-bot-keys ${line.keys.length !== 2 ? "is-column" : ""}`}>
                    {line.keys.map((key, keyIndex) => {
                      const pressed = line.pressed === keyIndex && index < shown - 1;
                      return (
                        <i key={key[1]} className={pressed ? "is-pressed" : ""}>
                          {pressed && <Check />}
                          {key[isRu ? 0 : 1]}
                        </i>
                      );
                    })}
                  </span>
                )}
              </li>
            ))}
            {typing && (
              <li className="h-bot-line is-bot is-typing">
                <p>
                  <i />
                  <i />
                  <i />
                </p>
              </li>
            )}
          </ol>
        </div>
      </div>
    </section>
  );
}
