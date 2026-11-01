import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export type Theme = "system" | "light" | "dark";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

/** "system" leaves data-theme unset so the prefers-color-scheme media query in App.css decides. */
function applyTheme(theme: Theme) {
  if (theme === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}

/** Colour theme choice; remembered on this machine like the number format. */
export const useThemeStore = create<ThemeState>()(persist((set) => ({
  theme: "system",
  setTheme: (theme) => { set({ theme }); },
}), {
  name: "cad2maxwell-theme",
  version: 1,
  storage: createJSONStorage(() => localStorage),
  partialize: (state) => ({ theme: state.theme }),
}));

applyTheme(useThemeStore.getState().theme);
useThemeStore.subscribe((state) => { applyTheme(state.theme); });
