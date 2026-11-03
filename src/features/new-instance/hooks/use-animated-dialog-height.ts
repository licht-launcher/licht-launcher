import { type RefObject, useEffect, useLayoutEffect, useRef } from "react";

const DURATION_MS = 200;

/**
 * Smooths the dialog's height when `key` changes (here, the popup's step).
 *
 * The dialog sizes itself to its content, so swapping the content would make
 * it jump. This measures the height before and after the swap and animates
 * between them; the dialog already clips its overflow while that runs. The
 * `anchor` is any element inside the dialog.
 */
export function useAnimatedDialogHeight(
  anchor: RefObject<HTMLElement | null>,
  key: unknown,
) {
  const lastHeight = useRef<number | null>(null);

  // Keeps the last known height up to date, whatever made it change.
  useEffect(() => {
    const dialog = dialogOf(anchor.current);
    if (dialog === null) {
      return;
    }
    const observer = new ResizeObserver(() => {
      lastHeight.current = dialog.offsetHeight;
    });
    observer.observe(dialog);
    return () => {
      observer.disconnect();
    };
  }, [anchor]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: runs when `key` changes.
  useLayoutEffect(() => {
    const dialog = dialogOf(anchor.current);
    if (dialog === null) {
      return;
    }
    const next = dialog.offsetHeight;
    const before = lastHeight.current;
    lastHeight.current = next;
    if (
      before === null ||
      Math.abs(before - next) < 1 ||
      typeof dialog.animate !== "function" ||
      prefersReducedMotion()
    ) {
      return;
    }
    dialog.animate([{ height: `${before}px` }, { height: `${next}px` }], {
      duration: DURATION_MS,
      easing: "ease-out",
    });
  }, [anchor, key]);
}

function prefersReducedMotion(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function dialogOf(anchor: HTMLElement | null): HTMLElement | null {
  return anchor?.closest<HTMLElement>("[data-slot='dialog-content']") ?? null;
}
