import { useEffect, useRef, useState } from "react";
import {
  ArrowUpRight,
  Check,
  Github,
  Globe2,
  MessageCircleHeart,
  Send,
  ShieldCheck,
  Star,
} from "lucide-react";
import { useLocale } from "../i18n";
import { emoteMoods } from "./EmoteStage";
import { useMotion } from "./useMotion";
import botAvatar from "../../website/public/betterfy-logo.png";

// The end of Home: BetterFy's Telegram, told by the bot itself. A chat window
// plays the real bot messages (services/auth-worker/src/copy.mjs) one after
// another while the section is on screen, with three emoticon stickers
// floating around it in the page's mood colour. Reduced motion shows the
// whole conversation at once.

type Line =
  | { from: "bot"; text: [string, string]; keys?: Array<[string, string]>; pressed?: number }
  | { from: "me"; text: [string, string] };

const conversation: Line[] = [
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
  },
];

const STEP = 1700;
const HOLD = 6500;

function useConversation(running: boolean) {
  const [shown, setShown] = useState(conversation.length);
  const [typing, setTyping] = useState(false);
  // Each time the chat comes into view the conversation starts over.
  useEffect(() => {
    if (running) setShown(0);
  }, [running]);
  useEffect(() => {
    if (!running) {
      setShown(conversation.length);
      setTyping(false);
      return undefined;
    }
    const done = shown >= conversation.length;
    const next = conversation[shown];
    // The bot "types" before each of its messages; the player's line just appears.
    setTyping(!done && next?.from === "bot");
    const timer = window.setTimeout(
      () => setShown((value) => (value >= conversation.length ? 0 : value + 1)),
      done ? HOLD : STEP,
    );
    return () => window.clearTimeout(timer);
  }, [running, shown]);
  return { shown, typing };
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
  const { shown, typing } = useConversation(animated && inView);
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
          <li>
            <ShieldCheck />
            {t(
              "Вход без паролей: подтверждаешь одной кнопкой в боте.",
              "No passwords: approve the sign-in with one tap in the bot.",
            )}
          </li>
          <li>
            <Star />
            {t(
              "Premium за Telegram Stars, от трёх дней.",
              "Premium with Telegram Stars, from three days.",
            )}
          </li>
          <li>
            <MessageCircleHeart />
            {t(
              "В @BeterHelp отвечают люди, которые делают BetterFy.",
              "At @BeterHelp the people who build BetterFy answer.",
            )}
          </li>
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

      <div className="h-bot-stage" ref={stage} aria-hidden="true">
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
          <ol>
            {conversation.slice(0, shown).map((line, index) => (
              <li key={index} className={`h-bot-line is-${line.from}`}>
                <p>{line.text[isRu ? 0 : 1]}</p>
                {line.from === "bot" && line.keys && (
                  <span className={`h-bot-keys ${line.keys.length > 2 ? "is-column" : ""}`}>
                    {line.keys.map((key, keyIndex) => (
                      <i
                        key={key[1]}
                        className={
                          line.pressed === keyIndex && index < shown - 1 ? "is-pressed" : ""
                        }
                      >
                        {line.pressed === keyIndex && index < shown - 1 && <Check />}
                        {key[isRu ? 0 : 1]}
                      </i>
                    ))}
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
