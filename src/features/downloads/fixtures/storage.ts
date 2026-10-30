import type { StorageSummary } from "@/lib/generated/StorageSummary";

export const storageSummary: StorageSummary = {
  diskName: "Local Disk (C:)",
  managedBytes: 20_000_000_000,
  categories: [
    { category: "instances", bytes: 12_000_000_000 },
    { category: "gameFiles", bytes: 4_500_000_000 },
    { category: "natives", bytes: 500_000_000 },
    { category: "java", bytes: 2_000_000_000 },
    { category: "cache", bytes: 750_000_000 },
    { category: "other", bytes: 250_000_000 },
  ],
};
