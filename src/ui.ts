export interface WindowInfo {
  id: number;
  app_name: string;
  app_pid: number;
  title: string;
  app_bundle_id: string | null;
  is_minimized: boolean;
  display_text: string;
  icon_data_url: string | null;
}

/**
 * Compute the next selected index given a navigation delta.
 * Wraps around at both ends.
 */
export function moveSelection(delta: number, currentIndex: number, total: number): number {
  if (total === 0) return 0;
  return (currentIndex + delta + total) % total;
}

/**
 * Build the list items for the results panel.
 * Returns the root <ul> element's children as a DocumentFragment.
 */
export function buildResultItems(
  windows: WindowInfo[],
  selectedIndex: number,
  document: Document
): DocumentFragment {
  const fragment = document.createDocumentFragment();

  if (windows.length === 0) {
    const li = document.createElement("li");
    li.className = "empty-state";
    li.textContent = "No windows found";
    fragment.appendChild(li);
    return fragment;
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

    const textWrap = document.createElement("span");
    textWrap.className = "result-text";

    const appSpan = document.createElement("span");
    appSpan.className = "result-app";
    appSpan.textContent = w.app_name;
    textWrap.appendChild(appSpan);

    if (w.title) {
      const titleSpan = document.createElement("span");
      titleSpan.className = "result-title";
      titleSpan.textContent = w.title;
      textWrap.appendChild(titleSpan);
    }

    li.appendChild(textWrap);

    if (w.is_minimized) {
      const minSpan = document.createElement("span");
      minSpan.className = "result-minimized";
      minSpan.textContent = "minimized";
      li.appendChild(minSpan);
    }

    fragment.appendChild(li);
  }

  return fragment;
}
