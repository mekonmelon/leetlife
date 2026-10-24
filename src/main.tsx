import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./game/game.css";

const container = document.getElementById("root");
if (!container) throw new Error("#root missing");

createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
