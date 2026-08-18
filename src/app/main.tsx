import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../styles/global.css";
import { App } from "./App";

const raiz = document.getElementById("root");
if (raiz) {
  createRoot(raiz).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
