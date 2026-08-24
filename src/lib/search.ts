import { invoke } from "@tauri-apps/api/core";

export type MediaKind = "video" | "image" | "audio";

export interface SearchHit {
  name: string;
  path: string;
  dir: string;
  kind: MediaKind;
  score: number;
}

export interface SearchResponse {
  id: number;
  hits: SearchHit[];
  elapsedMs: number;
  searched: number;
}

export interface IndexMeta {
  loaded: boolean;
  fileCount: number;
  dirCount: number;
  memoryBytes: number;
  builtAtUnix: number;
  problem: string | null;
}

let nextId = 0;
let latestId = 0;

/**
 * Run a search, returning `null` if a newer keystroke superseded it.
 *
 * There is deliberately no long debounce. The backend answers in a couple of
 * milliseconds, so a 150-200ms timer would *be* the latency the user feels —
 * it would account for most of the delay rather than avoiding it. Instead
 * every query carries an id: the backend abandons superseded work at its next
 * chunk boundary, and anything that still comes back late is dropped here.
 */
export async function searchFiles(
  query: string,
  kinds: MediaKind[],
  limit = 5000,
): Promise<SearchResponse | null> {
  const id = ++nextId;
  latestId = id;

  const res = await invoke<SearchResponse>("search", { id, query, kinds, limit });
  return res.id === latestId ? res : null;
}

let coalesceTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Collapse a burst of keystrokes into one search.
 *
 * 30ms — under two frames at 60Hz, so it is not perceptible, but enough that a
 * fast typist produces one search per word rather than one per letter.
 */
export function coalesce(fn: () => void, ms = 30): void {
  clearTimeout(coalesceTimer);
  coalesceTimer = setTimeout(fn, ms);
}

/** Open a file with whatever Windows uses for that type. */
export function openFile(path: string): Promise<void> {
  return invoke("open_file", { path });
}

/** Open File Explorer with this file selected. */
export function revealInExplorer(path: string): Promise<void> {
  return invoke("reveal_in_explorer", { path });
}

export function indexStatus(): Promise<IndexMeta> {
  return invoke<IndexMeta>("index_status");
}

export function formatCount(n: number): string {
  return n.toLocaleString("vi-VN");
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatWhen(unix: number): string {
  if (!unix) return "";
  return new Date(unix * 1000).toLocaleString("vi-VN");
}
