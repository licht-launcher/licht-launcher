export type PaginationItem = number | "start-gap" | "end-gap";

/** Most page buttons shown at once, gaps included. */
const MAX_ITEMS = 7;

/**
 * Pages to show for `page` out of `pageCount`: always the first and last page,
 * the current page with one neighbor on each side, and gaps for the rest.
 */
export function paginationItems(
  page: number,
  pageCount: number,
): PaginationItem[] {
  if (pageCount <= MAX_ITEMS) {
    return range(1, pageCount);
  }
  const start = Math.max(2, Math.min(page - 1, pageCount - 4));
  const end = Math.min(pageCount - 1, Math.max(page + 1, 5));
  return [
    1,
    ...(start > 2 ? (["start-gap"] as const) : []),
    ...range(start, end),
    ...(end < pageCount - 1 ? (["end-gap"] as const) : []),
    pageCount,
  ];
}

function range(from: number, to: number): number[] {
  return Array.from({ length: Math.max(0, to - from + 1) }, (_, i) => from + i);
}
