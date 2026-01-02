import { getCurrentWindow } from "@tauri-apps/api/window";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { listen } from "@tauri-apps/api/event";

const currentWindow = getCurrentWindow();

listen("tauri://blur", () => {
  currentWindow.hide();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
