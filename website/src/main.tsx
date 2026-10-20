import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// The same type and styles as the app, so the website reads as the app.
import "@fontsource/manrope/latin-400.css";
import "@fontsource/manrope/cyrillic-400.css";
import "@fontsource/manrope/latin-500.css";
import "@fontsource/manrope/cyrillic-500.css";
import "@fontsource/manrope/latin-600.css";
import "@fontsource/manrope/cyrillic-600.css";
import "@fontsource/manrope/latin-700.css";
import "@fontsource/manrope/cyrillic-700.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/cyrillic-400.css";
import "../../src/studio/studio-tokens.css";
import "../../src/studio/studio.css";
import "../../src/studio/studio-polish.css";
import "../../src/studio/studio-finish.css";
import "../../src/studio/auth-id.css";
import "../../src/studio/studio-motion.css";
import "../../src/studio/auth-motion.css";
import "../../src/studio/studio-type.css";
import "../../src/studio/home.css";
import "../../src/spotlight";
import Site from "./Site";
import "./site.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Site />
  </StrictMode>,
);
