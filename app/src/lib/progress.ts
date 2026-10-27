// Shared by the panel's indexing notice and the onboarding ring.

import type { IndexProgress } from "./ipc/bindings";

/** The first scan's progress as a whole percentage, by bytes (files differ wildly in size). */
export function indexPercent(progress: IndexProgress): number {
  const { bytes_done, bytes_total } = progress;
  return Math.floor(bytes_total > 0 ? Math.min(100, (bytes_done / bytes_total) * 100) : 0);
}
