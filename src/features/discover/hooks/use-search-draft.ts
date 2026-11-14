import { useEffect, useRef, useState } from "react";

const SEARCH_DELAY_MS = 300;

/**
 * The search box text, sent to `onSearch` once typing pauses. A query that
 * changes from outside (back/forward) replaces the text and drops any pending
 * send, so history navigation is never undone by a late keystroke.
 */
export function useSearchDraft(
  query: string,
  onSearch: (query: string) => void,
) {
  const [draft, setDraft] = useState(query);
  const timer = useRef<number | undefined>(undefined);
  const lastSent = useRef(query);

  useEffect(() => {
    if (query === lastSent.current) {
      return;
    }
    window.clearTimeout(timer.current);
    lastSent.current = query;
    setDraft(query);
  }, [query]);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  function change(value: string) {
    setDraft(value);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      const next = value.trim();
      if (next !== lastSent.current) {
        lastSent.current = next;
        onSearch(next);
      }
    }, SEARCH_DELAY_MS);
  }

  return { draft, change };
}
