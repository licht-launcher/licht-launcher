import type { DiscoverSearch } from "@/features/discover/lib/discover-search";

let last: DiscoverSearch = {};

/** The Discover list's search this session, so going back to it restores it. */
export function lastDiscoverSearch(): DiscoverSearch {
  return last;
}

export function rememberDiscoverSearch(search: DiscoverSearch) {
  last = search;
}

/** Starts the session over with no search; tests share one module instance. */
export function forgetDiscoverSearch() {
  last = {};
}
