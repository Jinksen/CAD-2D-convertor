import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import { DEFAULT_NUMBER_FORMAT, type NumberFormat } from "../lib/formatting";

interface FormatState {
  format: NumberFormat;
  setFormat: (patch: Partial<NumberFormat>) => void;
  resetFormat: () => void;
}

/** User display preferences for numbers and units; remembered on this machine. */
export const useFormatStore = create<FormatState>()(persist((set) => ({
  format: DEFAULT_NUMBER_FORMAT,
  setFormat: (patch) => { set((state) => ({ format: { ...state.format, ...patch } })); },
  resetFormat: () => { set({ format: DEFAULT_NUMBER_FORMAT }); },
}), {
  name: "cad2maxwell-number-format",
  version: 1,
  storage: createJSONStorage(() => localStorage),
  partialize: (state) => ({ format: state.format }),
  // Settings saved by an older build keep their values and gain new defaults.
  merge: (persisted, current) => ({
    ...current,
    format: { ...DEFAULT_NUMBER_FORMAT, ...(persisted as Partial<FormatState> | undefined)?.format },
  }),
}));

export function useNumberFormat(): NumberFormat {
  return useFormatStore((state) => state.format);
}
