import { type ComponentProps, useId } from "react";

import { cn } from "@/lib/utils";

import { cardStyles } from "./card.styles";

export function Card({
  title,
  className,
  ...props
}: ComponentProps<"section">) {
  const titleId = useId();
  const content = (
    <section
      data-slot="card"
      aria-labelledby={title && !props["aria-label"] ? titleId : undefined}
      className={cn(cardStyles.root, className)}
      {...props}
    />
  );

  return title ? (
    <div className={cardStyles.wrapper}>
      <h3 id={titleId} className={cardStyles.title}>
        {title}
      </h3>
      {content}
    </div>
  ) : (
    content
  );
}
