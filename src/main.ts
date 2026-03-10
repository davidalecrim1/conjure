import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface WindowInfo {
  id: number;
  app_name: string;
  app_pid: number;
  title: string;
  app_bundle_id: string | null;
  is_minimized: boolean;
  display_text: string;
  icon_data_url: string | null;
}

const appWindow = getCurrentWindow();
let windows: WindowInfo[] = [];
let selectedIndex = 0;

const searchEl = document.getElementById("search") as HTMLInputElement;
const resultsEl = document.getElementById("results") as HTMLUListElement;

async function loadWindows(query: string) {
  try {
    windows = await invoke<WindowInfo[]>("list_windows", { query });
  } catch {
    windows = [];
  }
  selectedIndex = 0;
  render();
}

function render() {
  resultsEl.innerHTML = "";

  if (windows.length === 0) {
    const li = document.createElement("li");
    li.className = "empty-state";
    li.textContent = "No windows found";
    resultsEl.appendChild(li);
    return;
  }

  for (let i = 0; i < windows.length; i++) {
    const w = windows[i];
    const li = document.createElement("li");
    li.className = `result-item${i === selectedIndex ? " selected" : ""}`;
    li.dataset.index = String(i);

    if (w.icon_data_url) {
      const img = document.createElement("img");
      img.className = "result-icon";
      img.src = w.icon_data_url;
      li.appendChild(img);
    }

    const appSpan = document.createElement("span");
    appSpan.className = "result-app";
    appSpan.textContent = w.app_name;

    const sep = document.createElement("span");
    sep.className = "result-separator";

    const titleSpan = document.createElement("span");
    titleSpan.className = "result-title";
    titleSpan.textContent = w.title || w.app_name;

    li.appendChild(appSpan);
    if (w.title) {
      li.appendChild(sep);
      li.appendChild(titleSpan);
    }

    if (w.is_minimized) {
      const minSpan = document.createElement("span");
      minSpan.className = "result-minimized";
      minSpan.textContent = "minimized";
      li.appendChild(minSpan);
    }

    resultsEl.appendChild(li);
  }

  scrollSelectedIntoView();
}

function scrollSelectedIntoView() {
  const item = resultsEl.querySelector(".selected");
  item?.scrollIntoView({ block: "nearest" });
}

function moveSelection(delta: number) {
  if (windows.length === 0) return;
  selectedIndex = (selectedIndex + delta + windows.length) % windows.length;
  render();
}

async function activateSelected() {
  const w = windows[selectedIndex];
  if (!w) return;
  try {
    await invoke("activate_window", { windowId: w.id, appPid: w.app_pid });
  } catch (e) {
    console.error("activate_window failed:", e);
  }
  await hidePalette();
}

async function hidePalette() {
  isOpen = false;
  searchEl.value = "";
  windows = [];
  resultsEl.innerHTML = "";
  await invoke("hide_palette");
}

async function onPaletteOpen() {
  searchEl.value = "";
  selectedIndex = 0;
  await invoke("refresh_windows");
  await loadWindows("");
  searchEl.focus();
}

// Keyboard handler -- all navigation is keyboard-driven.
// capture:true ensures we intercept keys before the focused input element does.
document.addEventListener("keydown", async (e) => {
  if (e.key === "Tab") {
    e.preventDefault();
    moveSelection(e.shiftKey ? -1 : 1);
    return;
  }

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      moveSelection(1);
      break;
    case "ArrowUp":
      e.preventDefault();
      moveSelection(-1);
      break;
    case "Enter":
      e.preventDefault();
      await activateSelected();
      break;
    case "Escape":
      e.preventDefault();
      if (searchEl.value) {
        searchEl.value = "";
        loadWindows("");
      } else {
        await hidePalette();
      }
      break;
  }
}, { capture: true });

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
searchEl.addEventListener("input", () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => loadWindows(searchEl.value), 120);
});

// Dismiss when window loses focus (user Cmd+Tabs away).
// isOpen gates this so macOS's transient blur event during window activation
// (emitted before focus actually lands on the palette) doesn't immediately
// close it. The flag is set synchronously in the palette-opened listener,
// which runs before the focus events reach JS.
let isOpen = false;

appWindow.onFocusChanged(({ payload: focused }) => {
  if (!focused && isOpen) {
    hidePalette();
  }
});

// Listen for palette-opened event from Rust (fired on hotkey press)
listen("palette-opened", () => {
  isOpen = true;
  onPaletteOpen();
});

// Initial load when DOM is ready
document.addEventListener("DOMContentLoaded", () => {
  onPaletteOpen();
});
