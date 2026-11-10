import { ChevronLeftIcon, ChevronRightIcon, EllipsisIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { paginationStyles } from "./pagination.styles";
import { paginationItems } from "./pagination-items";

/** Previous/next and numbered page buttons; renders nothing for one page. */
export function Pagination({
  page,
  pageCount,
  onPageChange,
  disabled = false,
}: {
  page: number;
  pageCount: number;
  onPageChange: (page: number) => void;
  disabled?: boolean;
}) {
  if (pageCount <= 1) {
    return null;
  }

  return (
    <nav aria-label="Pagination" className={paginationStyles.root}>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Previous page"
        disabled={disabled || page <= 1}
        onClick={() => onPageChange(page - 1)}
      >
        <ChevronLeftIcon aria-hidden />
      </Button>
      <ul className={paginationStyles.list}>
        {paginationItems(page, pageCount).map((item) =>
          typeof item === "number" ? (
            <li key={item}>
              <Button
                type="button"
                variant={item === page ? "outline" : "ghost"}
                size="sm"
                className={paginationStyles.page}
                aria-label={`Page ${item}`}
                aria-current={item === page ? "page" : undefined}
                disabled={disabled}
                onClick={() => onPageChange(item)}
              >
                {item}
              </Button>
            </li>
          ) : (
            <li key={item} aria-hidden className={paginationStyles.gap}>
              <EllipsisIcon className={paginationStyles.gapIcon} />
            </li>
          ),
        )}
      </ul>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Next page"
        disabled={disabled || page >= pageCount}
        onClick={() => onPageChange(page + 1)}
      >
        <ChevronRightIcon aria-hidden />
      </Button>
    </nav>
  );
}
