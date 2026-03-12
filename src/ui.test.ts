import { describe, it, expect } from "vitest";
import { JSDOM } from "jsdom";
import { moveSelection, buildResultItems, WindowInfo } from "./ui";

function makeWindow(partial: Partial<WindowInfo> = {}): WindowInfo {
  return {
    id: 1,
    app_name: "TestApp",
    app_pid: 100,
    title: "test title",
    app_bundle_id: null,
    is_minimized: false,
    display_text: "TestApp - test title",
    icon_data_url: null,
    ...partial,
  };
}

function makeDocument(): Document {
  return new JSDOM("<!DOCTYPE html>").window.document;
}

describe("moveSelection", () => {
  it("moves forward", () => {
    expect(moveSelection(1, 0, 3)).toBe(1);
  });

  it("wraps at end", () => {
    expect(moveSelection(1, 2, 3)).toBe(0);
  });

  it("moves backward and wraps", () => {
    expect(moveSelection(-1, 0, 3)).toBe(2);
  });

  it("returns 0 when total is 0", () => {
    expect(moveSelection(1, 0, 0)).toBe(0);
  });
});

describe("buildResultItems", () => {
  it("renders empty state when windows is empty", () => {
    const doc = makeDocument();
    const fragment = buildResultItems([], 0, doc);
    const ul = doc.createElement("ul");
    ul.appendChild(fragment);
    expect(ul.querySelector(".empty-state")).not.toBeNull();
    expect(ul.querySelectorAll(".result-item").length).toBe(0);
  });

  it("applies selected class to item at selectedIndex", () => {
    const doc = makeDocument();
    const windows = [makeWindow({ id: 1 }), makeWindow({ id: 2 }), makeWindow({ id: 3 })];
    const fragment = buildResultItems(windows, 1, doc);
    const ul = doc.createElement("ul");
    ul.appendChild(fragment);
    const items = ul.querySelectorAll(".result-item");
    expect(items[0].classList.contains("selected")).toBe(false);
    expect(items[1].classList.contains("selected")).toBe(true);
    expect(items[2].classList.contains("selected")).toBe(false);
  });

  it("renders minimized badge for minimized windows", () => {
    const doc = makeDocument();
    const windows = [makeWindow({ is_minimized: true })];
    const fragment = buildResultItems(windows, 0, doc);
    const ul = doc.createElement("ul");
    ul.appendChild(fragment);
    expect(ul.querySelector(".result-minimized")).not.toBeNull();
  });

  it("does not render minimized badge for normal windows", () => {
    const doc = makeDocument();
    const windows = [makeWindow({ is_minimized: false })];
    const fragment = buildResultItems(windows, 0, doc);
    const ul = doc.createElement("ul");
    ul.appendChild(fragment);
    expect(ul.querySelector(".result-minimized")).toBeNull();
  });
});
