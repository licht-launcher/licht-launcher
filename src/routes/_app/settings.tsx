import { createFileRoute } from "@tanstack/react-router";
import { PageTransition } from "@/components/ui/page-transition";
import { SECTIONS, type SectionId } from "@/features/settings/model/sections";
import { SettingsPage } from "@/features/settings/settings-page";

export type SettingsSearch = {
  section: SectionId;
};

function parseSection(value: unknown): SectionId {
  return SECTIONS.find((item) => item.id === value)?.id ?? "account";
}

export const Route = createFileRoute("/_app/settings")({
  validateSearch: (search: Record<string, unknown>): SettingsSearch => ({
    section: parseSection(search.section),
  }),
  component: SettingsRoute,
});

function SettingsRoute() {
  const { section } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PageTransition>
      <SettingsPage
        section={section}
        onSectionChange={(next) => void navigate({ search: { section: next } })}
      />
    </PageTransition>
  );
}
