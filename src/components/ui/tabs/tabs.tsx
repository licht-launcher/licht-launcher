import type { VariantProps } from "class-variance-authority";
import { cn } from "cn";
import { Tabs as TabsPrimitive } from "radix-ui";
import * as React from "react";

import { tabsListVariants, tabsStyles } from "./tabs.styles";

function Tabs({
  className,
  orientation = "horizontal",
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      data-orientation={orientation}
      className={cn(tabsStyles.root, className)}
      {...props}
    />
  );
}

function TabsList({
  className,
  variant = "default",
  children,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List> &
  VariantProps<typeof tabsListVariants>) {
  const indicatorRef = React.useRef<HTMLSpanElement>(null);
  React.useLayoutEffect(() => {
    const indicator = indicatorRef.current;
    const list = indicator?.parentElement;
    if (!indicator || !list || variant !== "sliding") return;

    const resizeObserver = new ResizeObserver(updateIndicator);
    function updateIndicator() {
      if (!indicator || !list) return;
      const triggers = list.querySelectorAll<HTMLElement>(
        "[data-slot='tabs-trigger']",
      );
      for (const trigger of triggers) resizeObserver.observe(trigger);
      const selected = list.querySelector<HTMLElement>(
        "[data-slot='tabs-trigger'][data-state='active']",
      );
      indicator.style.setProperty(
        "--tabs-indicator-x",
        `${selected?.offsetLeft ?? 0}px`,
      );
      indicator.style.setProperty(
        "--tabs-indicator-width",
        `${selected?.offsetWidth ?? 0}px`,
      );
      if (indicator.dataset.ready !== "true") {
        // Resolve the first size before enabling motion, avoiding a grow-from-zero entry.
        indicator.getBoundingClientRect();
        indicator.dataset.ready = "true";
      }
    }

    const mutationObserver = new MutationObserver(updateIndicator);
    mutationObserver.observe(list, {
      attributes: true,
      attributeFilter: ["data-state"],
      childList: true,
      subtree: true,
    });
    resizeObserver.observe(list);
    updateIndicator();
    return () => {
      resizeObserver.disconnect();
      mutationObserver.disconnect();
    };
  }, [variant]);

  return (
    <TabsPrimitive.List
      data-slot="tabs-list"
      data-variant={variant}
      className={cn(tabsListVariants({ variant }), className)}
      {...props}
    >
      {variant === "sliding" && (
        <span
          aria-hidden
          ref={indicatorRef}
          data-slot="tabs-indicator"
          className={tabsStyles.indicator}
        />
      )}
      {children}
    </TabsPrimitive.List>
  );
}

function TabsTrigger({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      className={cn(tabsStyles.trigger, className)}
      {...props}
    />
  );
}

function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      data-slot="tabs-content"
      className={cn(tabsStyles.content, className)}
      {...props}
    />
  );
}

export { Tabs, TabsContent, TabsList, TabsTrigger, tabsListVariants };
