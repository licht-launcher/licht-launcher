import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { formSectionStyles } from "./form-section.styles";

/** A titled block of the create form: icon, title, optional description, then content. */
export function FormSection({
  icon: Icon,
  title,
  description,
  divided = true,
  children,
}: {
  icon: LucideIcon;
  title: string;
  description?: string;
  /** Draws a divider above the section; the first one omits it. */
  divided?: boolean;
  children: ReactNode;
}) {
  return (
    <section
      className={cn(
        formSectionStyles.root,
        divided && formSectionStyles.rootDivided,
      )}
    >
      <div className={formSectionStyles.header}>
        <div className={formSectionStyles.iconWell}>
          <Icon aria-hidden className={formSectionStyles.icon} />
        </div>
        <div>
          <h3 className={formSectionStyles.title}>{title}</h3>
          {description === undefined ? null : (
            <p className={formSectionStyles.description}>{description}</p>
          )}
        </div>
      </div>
      {children}
    </section>
  );
}
