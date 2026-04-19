import { describe, it, expect, beforeEach } from "vitest";
import { getStoredTheme, setTheme, applyTheme } from "./theme";

describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark", "light");
  });

  it("defaults to system theme", () => {
    expect(getStoredTheme()).toBe("system");
  });

  it("stores and retrieves theme preference", () => {
    setTheme("dark");
    expect(getStoredTheme()).toBe("dark");
  });

  it("applies dark class to html element", () => {
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.classList.contains("light")).toBe(false);
  });

  it("applies light class to html element", () => {
    applyTheme("light");
    expect(document.documentElement.classList.contains("light")).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("system mode removes both classes", () => {
    applyTheme("dark");
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.classList.contains("light")).toBe(false);
  });
});
