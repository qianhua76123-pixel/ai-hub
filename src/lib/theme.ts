type Theme = "system" | "light" | "dark";

const THEME_KEY = "ai-hub-theme";

export function getStoredTheme(): Theme {
  return (localStorage.getItem(THEME_KEY) as Theme) || "system";
}

export function setTheme(theme: Theme) {
  localStorage.setItem(THEME_KEY, theme);
  applyTheme(theme);
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.remove("dark", "light");

  if (theme === "dark") {
    root.classList.add("dark");
  } else if (theme === "light") {
    root.classList.add("light");
  }
  // "system" = no class, CSS media query handles it
}

// Initialize on load
export function initTheme() {
  applyTheme(getStoredTheme());
}
