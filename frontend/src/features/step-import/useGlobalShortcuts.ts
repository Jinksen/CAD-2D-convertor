import { useEffect, useEffectEvent } from "react";

/** Shortcuts are ignored while the user types into a field. */
function isTyping(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest("input, textarea, select, [contenteditable]") !== null;
}

/**
 * Window-level keyboard shortcuts. The handler returns true when it consumed the key;
 * keys are skipped while typing, with Alt held, or when another handler already handled them.
 */
export function useGlobalShortcuts(handle: (key: string, ctrl: boolean, event: KeyboardEvent) => boolean) {
  const onKeyDown = useEffectEvent((event: KeyboardEvent) => {
    if (event.defaultPrevented || event.altKey || isTyping(event.target)) return;
    if (handle(event.key.toLowerCase(), event.ctrlKey || event.metaKey, event)) event.preventDefault();
  });
  useEffect(() => {
    window.addEventListener("keydown", onKeyDown);
    return () => { window.removeEventListener("keydown", onKeyDown); };
  }, []);
}
