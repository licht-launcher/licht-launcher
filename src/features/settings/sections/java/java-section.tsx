import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { Input } from "@/components/ui/input";
import { useJavaReady } from "@/features/settings/hooks/use-java-ready";
import { useToastError } from "@/features/settings/hooks/use-toast-error";
import type { Draft } from "@/features/settings/model/draft";
import {
  JAVA_MAJORS,
  type JavaField,
} from "@/features/settings/model/sections";
import { JavaMajorRow } from "@/features/settings/sections/java/java-major-row";
import {
  browseJava,
  detectJavaInstallations,
  installRecommendedJava,
  runtimeJava,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function JavaSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const queryClient = useQueryClient();
  const javaReady = useJavaReady(true);

  const runtime = useQuery({
    queryKey: ["runtime-java"],
    queryFn: runtimeJava,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  useToastError(runtime.error);

  const detections = useQuery({
    queryKey: ["java-installations"],
    queryFn: detectJavaInstallations,
    enabled: javaReady,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  useToastError(detections.error);

  const install = useMutation({
    mutationFn: installRecommendedJava,
    onSuccess: (path, major) => {
      if (path === null) return;
      const row = JAVA_MAJORS.find((item) => item.major === major);
      if (!row) {
        return;
      }
      onChange({ [row.field]: path });
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const browseExecutable = useMutation({
    mutationFn: browseJava,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  function browseFor(field: JavaField) {
    browseExecutable.mutate(draft[field], {
      onSuccess: (path) => {
        if (path) {
          onChange({ [field]: path });
        }
      },
    });
  }

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-4">
        {JAVA_MAJORS.map((row) => (
          <JavaMajorRow
            key={row.field}
            row={row}
            value={draft[row.field]}
            found={detections.data?.[row.field] ?? []}
            runtimePath={runtime.data?.[row.field] ?? null}
            checkReady={javaReady}
            detectionsPending={detections.isPending}
            busy={install.isPending}
            onChange={(path) => onChange({ [row.field]: path })}
            onInstall={() => install.mutate(row.major)}
            onRefreshDetections={() => {
              void detections.refetch();
            }}
            onBrowse={() => browseFor(row.field)}
          />
        ))}
      </section>

      <section className="flex flex-col gap-4 border-t border-border pt-6">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Java arguments</h3>
          <p id="jvm-arguments-hint" className="text-sm text-muted-foreground">
            Extra JVM flags, separated by spaces.
          </p>
        </div>
        <Input
          id="jvm-arguments"
          placeholder="-XX:+UseG1GC"
          aria-label="Java arguments"
          aria-describedby="jvm-arguments-hint"
          value={draft.jvmArguments}
          onChange={(event) => onChange({ jvmArguments: event.target.value })}
        />
      </section>
    </div>
  );
}
