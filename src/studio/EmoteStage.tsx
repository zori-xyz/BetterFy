import { useEffect, useState, type CSSProperties } from "react";
import { ChevronRight, Pause, Play } from "lucide-react";
import { useLocale } from "../i18n";
import { useMotion } from "./useMotion";
import kitty from "../assets/dota-emoticons/kitty-jug.gif";
import kittyStill from "../assets/dota-emoticons/kitty-jug.png";
import brain from "../assets/dota-emoticons/big-brain.gif";
import brainStill from "../assets/dota-emoticons/big-brain.png";
import creep from "../assets/dota-emoticons/creepdance.gif";
import creepStill from "../assets/dota-emoticons/creepdance.png";
import puck from "../assets/dota-emoticons/puckchamp.gif";
import puckStill from "../assets/dota-emoticons/puckchamp.png";
import marci from "../assets/dota-emoticons/marci-omnom.gif";
import marciStill from "../assets/dota-emoticons/marci-omnom.png";
import pog from "../assets/dota-emoticons/poghanim.gif";
import pogStill from "../assets/dota-emoticons/poghanim.png";
import swag from "../assets/dota-emoticons/swaghanim.gif";
import swagStill from "../assets/dota-emoticons/swaghanim.png";

export const emoteMoods = [
  { name: "Kitty Jug", image: kitty, still: kittyStill, color: "#f2b86e", ink: "#97571b", rgb: "242 184 110", light: "#ffe2b8" },
  { name: "PuckChamp", image: puck, still: puckStill, color: "#70cefb", ink: "#186c9a", rgb: "112 206 251", light: "#c4edff" },
  { name: "Marci Omnom", image: marci, still: marciStill, color: "#ecab89", ink: "#955034", rgb: "236 171 137", light: "#ffe0c9" },
  { name: "Creep Dance", image: creep, still: creepStill, color: "#b6d978", ink: "#526d20", rgb: "182 217 120", light: "#e2f4ba" },
  { name: "Big Brain", image: brain, still: brainStill, color: "#b7a3f5", ink: "#6f42a4", rgb: "183 163 245", light: "#e4d9ff" },
  { name: "Poghanim", image: pog, still: pogStill, color: "#c7a5e8", ink: "#79529d", rgb: "199 165 232", light: "#efdcff" },
  { name: "Swaghanim", image: swag, still: swagStill, color: "#a7b3eb", ink: "#4e60a1", rgb: "167 179 235", light: "#dce3ff" },
] as const;

export function emoteStyle(index: number): CSSProperties {
  const mood = emoteMoods[index];
  return { "--mood-color": mood.color, "--mood-ink": mood.ink, "--mood-rgb": mood.rgb, "--mood-light": mood.light } as CSSProperties;
}

export default function EmoteStage({ motion, active, onChange }: { motion: boolean; active: number; onChange: (index: number) => void }) {
  const { isRu } = useLocale();
  const { animated, visible } = useMotion(motion);
  const [paused, setPaused] = useState(false);
  const [stopped, setStopped] = useState(false);
  useEffect(() => {
    if (!animated || !visible || paused || stopped) return;
    const timer = window.setTimeout(() => onChange((active + 1) % emoteMoods.length), 7500);
    return () => window.clearTimeout(timer);
  }, [active, animated, visible, paused, stopped, onChange]);
  return <div className="s-emote-stage" onMouseEnter={() => setPaused(true)} onMouseLeave={() => setPaused(false)} onFocus={() => setPaused(true)} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPaused(false); }}>
    <div className="s-emote-portrait" aria-hidden="true">{emoteMoods.map((mood, index) => <img key={mood.name} className={active === index ? "is-active" : ""} src={active === index && animated && visible && !stopped && !paused ? mood.image : mood.still} alt="" width="96" height="96" />)}</div>
    <div className="s-emote-tools">
      <button className="s-icon" onClick={() => onChange((active + 1) % emoteMoods.length)} aria-label={isRu ? "Следующий эмодзи" : "Next emoticon"}><ChevronRight /></button>
      {animated && <button className="s-icon" onClick={() => setStopped(value => !value)} aria-label={isRu ? (stopped ? "Продолжить анимацию эмодзи" : "Остановить анимацию эмодзи") : (stopped ? "Resume emoticon animation" : "Pause emoticon animation")} aria-pressed={stopped}>{stopped ? <Play /> : <Pause />}</button>}
    </div>
  </div>;
}
