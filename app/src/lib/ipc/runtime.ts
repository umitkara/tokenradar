// Whether the page runs inside the app. The Vite dev server in a plain browser has no IPC;
// pages then render with fixtures or defaults.
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
