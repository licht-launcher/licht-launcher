import {
  type QueryClient,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

export const MEMORY_PREVIEW_SIZES_GIB = [4, 8, 16, 32, 64, 128] as const;
export const MEMORY_PREVIEW_QUERY_KEY = ["dev", "memory-preview"] as const;

type MemoryPreview = {
  totalMb: number | null;
  allocationMb: number | null;
};

const realMemory: MemoryPreview = { totalMb: null, allocationMb: null };

export function setMemoryPreview(client: QueryClient, totalMb: number | null) {
  if (!import.meta.env.DEV) return;
  if (
    totalMb !== null &&
    !MEMORY_PREVIEW_SIZES_GIB.some((gib) => gib * 1024 === totalMb)
  ) {
    throw new Error("Unsupported RAM preview size");
  }
  client.setQueryData<MemoryPreview>(MEMORY_PREVIEW_QUERY_KEY, {
    totalMb,
    allocationMb: null,
  });
}

export function useMemoryPreview() {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: MEMORY_PREVIEW_QUERY_KEY,
    queryFn: (): MemoryPreview => realMemory,
    initialData: realMemory,
    enabled: false,
    gcTime: Infinity,
  });
  const preview = import.meta.env.DEV ? query.data : realMemory;
  function setAllocationMb(allocationMb: number, minimumMb: number) {
    if (!import.meta.env.DEV) return;
    client.setQueryData<MemoryPreview>(MEMORY_PREVIEW_QUERY_KEY, (current) => {
      if (!current || current.totalMb === null) return current;
      if (
        !Number.isSafeInteger(allocationMb) ||
        allocationMb < minimumMb ||
        allocationMb > current.totalMb
      ) {
        throw new Error(
          "Memory preview allocation is outside the simulated RAM range",
        );
      }
      return { ...current, allocationMb };
    });
  }
  return { ...preview, setAllocationMb };
}
