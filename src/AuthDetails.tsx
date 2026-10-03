import { useEffect, useState } from "react";
import avatar from "./assets/betterfy-avatar.png";

// Small pieces of the sign-in screens: a preview of the bot message that
// carries the code, digits that roll into place, and the burst behind the
// success check.

export function BotHint({ isRu }: { isRu: boolean }) {
  const [typing, setTyping] = useState(true);
  useEffect(() => {
    const timer = window.setTimeout(() => setTyping(false), 1300);
    return () => window.clearTimeout(timer);
  }, []);
  return (
    <div className="auth-bot-hint" aria-hidden="true">
      <img className="auth-bot-avatar" src={avatar} alt="" />
      <div className="auth-bot-bubble">
        <strong>BetterFy Bot</strong>
        {typing ? (
          <span className="auth-bot-typing">
            <i />
            <i />
            <i />
          </span>
        ) : (
          <span className="auth-bot-message">
            {isRu ? "Код для входа:" : "Sign-in code:"}
            <b>
              <i>•</i>
              <i>•</i>
              <i>•</i> <i>•</i>
              <i>•</i>
              <i>•</i>
            </b>
          </span>
        )}
      </div>
    </div>
  );
}

export function RollingDigits({ value }: { value: string }) {
  return (
    <span className="rolling-digits" aria-label={value}>
      {[...value].map((digit, index) => (
        <span className="rolling-digit" key={`${index}-${digit}`} aria-hidden="true">
          <span style={{ animationDelay: `${index * 0.14}s` }}>
            {"0123456789".split("").map((item) => (
              <i key={item}>{item}</i>
            ))}
            <i>{digit}</i>
          </span>
        </span>
      ))}
    </span>
  );
}

export function SuccessBurst() {
  return (
    <span className="success-burst" aria-hidden="true">
      {Array.from({ length: 14 }, (_, index) => (
        <i
          key={index}
          style={{
            ["--angle" as string]: `${index * (360 / 14) + (index % 2) * 9}deg`,
            ["--distance" as string]: `${46 + (index % 3) * 12}px`,
            animationDelay: `${(index % 4) * 0.03}s`,
          }}
        />
      ))}
    </span>
  );
}
