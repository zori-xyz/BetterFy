import { useEffect, useState, type CSSProperties } from "react";
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

import aaaah from "../assets/dota-emoticons/aaaah.gif";
import aaaahStill from "../assets/dota-emoticons/aaaah.png";
import facepalm from "../assets/dota-emoticons/facepalm.gif";
import facepalmStill from "../assets/dota-emoticons/facepalm.png";
import frag from "../assets/dota-emoticons/fraghanim.gif";
import fragStill from "../assets/dota-emoticons/fraghanim.png";
import pudge from "../assets/dota-emoticons/pudgesteal.gif";
import pudgeStill from "../assets/dota-emoticons/pudgesteal.png";
import smug from "../assets/dota-emoticons/smughanim.gif";
import smugStill from "../assets/dota-emoticons/smughanim.png";

// Twelve moods, ordered so neighbours never share a colour family. A new
// mood's colours come from its own pixels: the dominant hue at 72% lightness
// (color), 33% (ink, light theme) and 89% (light).
export const emoteMoods = [
  {
    name: "Kitty Jug",
    image: kitty,
    still: kittyStill,
    color: "#f2b86e",
    ink: "#97571b",
    rgb: "242 184 110",
    light: "#ffe2b8",
  },
  {
    name: "PuckChamp",
    image: puck,
    still: puckStill,
    color: "#70cefb",
    ink: "#186c9a",
    rgb: "112 206 251",
    light: "#c4edff",
  },
  {
    name: "Fraghanim",
    image: frag,
    still: fragStill,
    color: "#db94c4",
    ink: "#7e2a62",
    rgb: "219 148 196",
    light: "#f1d5e8",
  },
  {
    name: "Creep Dance",
    image: creep,
    still: creepStill,
    color: "#b6d978",
    ink: "#526d20",
    rgb: "182 217 120",
    light: "#e2f4ba",
  },
  {
    name: "Smughanim",
    image: smug,
    still: smugStill,
    color: "#aa94db",
    ink: "#442a7e",
    rgb: "170 148 219",
    light: "#ddd5f1",
  },
  {
    name: "Facepalm",
    image: facepalm,
    still: facepalmStill,
    color: "#dba994",
    ink: "#7e432a",
    rgb: "219 169 148",
    light: "#f1ddd5",
  },
  {
    name: "Aaaah",
    image: aaaah,
    still: aaaahStill,
    color: "#94c0db",
    ink: "#2a5f7e",
    rgb: "148 192 219",
    light: "#d5e6f1",
  },
  {
    name: "Poghanim",
    image: pog,
    still: pogStill,
    color: "#c7a5e8",
    ink: "#79529d",
    rgb: "199 165 232",
    light: "#efdcff",
  },
  {
    name: "Pudge Steal",
    image: pudge,
    still: pudgeStill,
    color: "#93e28d",
    ink: "#298622",
    rgb: "147 226 141",
    light: "#d4f4d2",
  },
  {
    name: "Marci Omnom",
    image: marci,
    still: marciStill,
    color: "#ecab89",
    ink: "#955034",
    rgb: "236 171 137",
    light: "#ffe0c9",
  },
  {
    name: "Swaghanim",
    image: swag,
    still: swagStill,
    color: "#a7b3eb",
    ink: "#4e60a1",
    rgb: "167 179 235",
    light: "#dce3ff",
  },
  {
    name: "Big Brain",
    image: brain,
    still: brainStill,
    color: "#b7a3f5",
    ink: "#6f42a4",
    rgb: "183 163 245",
    light: "#e4d9ff",
  },
] as const;

export function emoteStyle(index: number): CSSProperties {
  const mood = emoteMoods[index];
  return {
    "--mood-color": mood.color,
    "--mood-ink": mood.ink,
    "--mood-rgb": mood.rgb,
    "--mood-light": mood.light,
  } as CSSProperties;
}

export default function EmoteStage({
  motion,
  active,
  onChange,
}: {
  motion: boolean;
  active: number;
  onChange: (index: number) => void;
}) {
  const { animated, visible } = useMotion(motion);
  const [paused, setPaused] = useState(false);
  useEffect(() => {
    if (!animated || !visible || paused) return;
    const timer = window.setTimeout(() => onChange((active + 1) % emoteMoods.length), 7500);
    return () => window.clearTimeout(timer);
  }, [active, animated, visible, paused, onChange]);
  return (
    <div
      className="s-emote-stage"
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setPaused(false);
      }}
    >
      <button
        type="button"
        className="s-emote-portrait"
        onClick={() => onChange((active + 1) % emoteMoods.length)}
        title={emoteMoods[active].name}
        aria-label={`${emoteMoods[active].name}: next mood`}
      >
        {emoteMoods.map((mood, index) => (
          <img
            key={mood.name}
            className={active === index ? "is-active" : ""}
            src={active === index && animated && visible && !paused ? mood.image : mood.still}
            alt=""
            width="96"
            height="96"
          />
        ))}
      </button>
    </div>
  );
}

// The scene behind the home hero: the current mood's emoticon blown up pixel
// for pixel into a faint mosaic. Each mood repaints the whole scene.
export function EmoteBackdrop({ active }: { active: number }) {
  return (
    <div className="s-hero-backdrop" aria-hidden="true">
      {emoteMoods.map((mood, index) => (
        <img
          key={mood.name}
          className={active === index ? "is-active" : ""}
          src={mood.still}
          alt=""
        />
      ))}
    </div>
  );
}
