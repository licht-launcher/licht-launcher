import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "cn";
import { CheckIcon, ChevronDownIcon } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverAnchor,
  PopoverContent,
} from "@/components/ui/popover";

export type ComboboxOption = { value: string; label: string };

const ROW_HEIGHT = 32;

/**
 * A select you can type in: the field shows the chosen option, typing filters
 * the list. The list is a popover above everything else; inside a dialog it
 * renders in that dialog, which keeps the wheel scrolling it, and it is
 * virtualized because option lists can be long.
 */
export function Combobox({
  options,
  value,
  onValueChange,
  id,
  "aria-label": ariaLabel,
  placeholder,
  emptyText = "No results.",
  disabled = false,
  className,
}: {
  options: ComboboxOption[];
  value: string;
  onValueChange: (value: string) => void;
  id?: string;
  "aria-label"?: string;
  placeholder?: string;
  emptyText?: string;
  disabled?: boolean;
  className?: string;
}) {
  const listId = useId();
  // State, not a ref: the list mounts after the popover opens, and the
  // virtualizer must render again once it has the element to measure.
  const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [container, setContainer] = useState<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState<string | null>(null);
  const [active, setActive] = useState(0);

  const selected = options.find((option) => option.value === value);
  const needle = (query ?? "").trim().toLowerCase();
  const visible =
    needle === ""
      ? options
      : options.filter((option) => option.label.toLowerCase().includes(needle));

  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => viewport,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => visible[index]?.value ?? index,
    overscan: 6,
  });

  useEffect(() => {
    if (open && visible.length > 0) {
      virtualizer.scrollToIndex(Math.min(active, visible.length - 1));
    }
  }, [open, active, visible.length, virtualizer]);

  function openList() {
    if (disabled || open) {
      return;
    }
    setContainer(
      inputRef.current?.closest<HTMLElement>("[data-slot='dialog-content']") ??
        null,
    );
    setActive(Math.max(0, options.indexOf(selected as ComboboxOption)));
    setOpen(true);
  }

  function close() {
    setOpen(false);
    setQuery(null);
  }

  function choose(option: ComboboxOption) {
    onValueChange(option.value);
    close();
  }

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          close();
        }
      }}
    >
      <PopoverAnchor asChild>
        <div className={cn("relative", className)}>
          <Input
            ref={inputRef}
            id={id}
            role="combobox"
            aria-label={ariaLabel}
            aria-expanded={open}
            aria-controls={listId}
            aria-activedescendant={
              open && visible.length > 0 ? `${listId}-${active}` : undefined
            }
            aria-autocomplete="list"
            autoComplete="off"
            disabled={disabled}
            placeholder={placeholder}
            className="pr-8"
            value={open && query !== null ? query : (selected?.label ?? "")}
            onFocus={openList}
            onClick={openList}
            onBlur={close}
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
              setOpen(true);
            }}
            onKeyDown={(event) => {
              switch (event.key) {
                case "ArrowDown":
                  event.preventDefault();
                  if (open) {
                    setActive((index) =>
                      Math.min(index + 1, visible.length - 1),
                    );
                  } else {
                    openList();
                  }
                  break;
                case "ArrowUp":
                  event.preventDefault();
                  setActive((index) => Math.max(index - 1, 0));
                  break;
                case "Enter": {
                  const option = visible[active];
                  if (open && option !== undefined) {
                    event.preventDefault();
                    choose(option);
                  }
                  break;
                }
                case "Escape":
                  if (open) {
                    // Keep a surrounding dialog open; only the list closes.
                    event.preventDefault();
                    event.stopPropagation();
                    close();
                  }
                  break;
              }
            }}
          />
          <ChevronDownIcon
            aria-hidden
            className={cn(
              "pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-muted-foreground transition-transform duration-200",
              open && "rotate-180",
            )}
          />
        </div>
      </PopoverAnchor>
      <PopoverContent
        container={container}
        align="start"
        collisionPadding={8}
        className="w-(--radix-popover-trigger-width) p-1"
        // The focus stays in the field; only the pointer uses the list.
        onOpenAutoFocus={(event) => {
          event.preventDefault();
        }}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
        }}
        onInteractOutside={(event) => {
          if (
            event.target instanceof Node &&
            inputRef.current?.contains(event.target)
          ) {
            event.preventDefault();
          }
        }}
      >
        {/* biome-ignore lint/a11y/noStaticElementInteractions: only keeps the focus in the field; the keyboard is handled by the input. */}
        <div
          id={listId}
          ref={setViewport}
          data-slot="combobox-viewport"
          // Keep the focus in the field while the list is used or scrolled.
          onMouseDown={(event) => {
            event.preventDefault();
          }}
          className="max-h-[min(16rem,var(--radix-popper-available-height))] overflow-y-auto text-sm"
        >
          {visible.length === 0 ? (
            <p className="px-2 py-2 text-muted-foreground">{emptyText}</p>
          ) : (
            <div
              role="listbox"
              aria-label={ariaLabel}
              className="relative w-full"
              // Rows are absolutely placed; only the visible window is rendered.
              style={{ height: virtualizer.getTotalSize() }}
            >
              {virtualizer.getVirtualItems().map((row) => {
                const option = visible[row.index];
                if (option === undefined) {
                  return null;
                }
                const isSelected = option.value === value;
                return (
                  // biome-ignore lint/a11y/useKeyWithClickEvents: the field owns the keyboard (aria-activedescendant); options are pointer targets.
                  <div
                    key={row.key}
                    id={`${listId}-${row.index}`}
                    role="option"
                    aria-selected={isSelected}
                    tabIndex={-1}
                    className={cn(
                      "absolute inset-x-0 top-0 flex cursor-pointer items-center justify-between rounded-md px-2 transition-colors",
                      row.index === active && "bg-muted",
                      isSelected && "font-medium",
                    )}
                    style={{
                      height: row.size,
                      transform: `translateY(${row.start}px)`,
                    }}
                    onMouseEnter={() => {
                      setActive(row.index);
                    }}
                    onClick={() => {
                      choose(option);
                    }}
                  >
                    {option.label}
                    {isSelected ? (
                      <CheckIcon aria-hidden className="size-4 text-primary" />
                    ) : null}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
