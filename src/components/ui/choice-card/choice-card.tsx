import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { choiceCardStyles } from "./choice-card.styles";

export type ChoiceCardIconTone = keyof typeof choiceCardStyles.iconTones;

export function ChoiceCard({
  title,
  icon,
  iconNeutral = false,
  iconTone = "instance",
  selected,
  disabled = false,
  onSelect,
  className,
  children,
}: {
  title: string;
  icon?: ReactNode;
  /** Neutral well for brand/loader images. */
  iconNeutral?: boolean;
  /** Accent well for Lucide icons when not neutral. */
  iconTone?: ChoiceCardIconTone;
  selected: boolean;
  disabled?: boolean;
  onSelect: () => void;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      className={cn(
        choiceCardStyles.root,
        selected
          ? choiceCardStyles.rootSelected
          : choiceCardStyles.rootUnselected,
        className,
      )}
      onClick={onSelect}
    >
      <span className={choiceCardStyles.body}>
        <div className={choiceCardStyles.header}>
          {icon !== undefined ? (
            <span
              className={cn(
                choiceCardStyles.icon,
                iconNeutral
                  ? choiceCardStyles.iconNeutral
                  : choiceCardStyles.iconTones[iconTone],
              )}
            >
              {icon}
            </span>
          ) : null}
          <span className={choiceCardStyles.title}>{title}</span>
        </div>
        {children}
      </span>
    </button>
  );
}
