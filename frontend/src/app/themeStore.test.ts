import { beforeEach, describe, expect, it } from "vitest";

import { useThemeStore } from "./themeStore";

describe("theme store", () => {
  beforeEach(() => { useThemeStore.getState().setTheme("system"); });

  it("defaults to the system theme without a data-theme attribute", () => {
    expect(useThemeStore.getState().theme).toBe("system");
    expect(document.documentElement).not.toHaveAttribute("data-theme");
  });

  it("sets data-theme and persists the choice", () => {
    useThemeStore.getState().setTheme("dark");
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(JSON.parse(localStorage.getItem("cad2maxwell-theme") ?? "{}")).toMatchObject({ state: { theme: "dark" } });
    useThemeStore.getState().setTheme("light");
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    useThemeStore.getState().setTheme("system");
    expect(document.documentElement).not.toHaveAttribute("data-theme");
  });
});
