import type { ReactNode } from "react";

import { Card } from "@/components/ui/card";

/**
 * A block of the create form, laid out like the settings sections: a small
 * title above a bordered card, with an optional muted hint at the top of it.
 */
export function FormSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <Card title={title}>
      {description === undefined ? null : (
        <p className="text-sm text-muted-foreground">{description}</p>
      )}
      {children}
    </Card>
  );
}
