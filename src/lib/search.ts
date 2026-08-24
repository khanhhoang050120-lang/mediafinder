import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type MediaKind = "video" | "image" | "audio";

export interface SearchHit {
  name: string;
  path: string;
  dir: string;
  kind: MediaKind;
  score: number;
  /** How many of the query's words this file actually contains. */
  matched: number;
  /** Position in the index, used to build the thumbnail URL. */
  index: number;
  size: number;
  /** Zero means the background reader has not reached this file yet. */
  width: number;
  height: number;
  durationMs: number;
}

export interface RelaxedInfo {
  totalTokens: number;
  bestMatched: number;
}

export interface SearchResponse {
  id: number;
  hits: SearchHit[];
  elapsedMs: number;
  searched: number;
  /**
   * Set when nothing matched the whole query and these are the closest partial
   * matches. Must be surfaced: partial results that look exact are worse than
   * no results, because the user stops looking.
   */
  relaxed: RelaxedInfo | null;
  /** Which index these results came from; pins thumbnail URLs to it. */
  epoch: number;
}

export interface IndexMeta {
  loaded: boolean;
  fileCount: number;
  dirCount: number;
  memoryBytes: number;
  builtAtUnix: number;
  problem: string | null;
}

export interface Filters {
  /** Shortest side in pixels; 0 disables. */
  minHeight: number;
  minDurationMs: number;
  maxDurationMs: number;
}

export const NO_FILTERS: Filters = {
  minHeight: 0,
  minDurationMs: 0,
  maxDurationMs: 0,
};

export interface EnrichStatus {
  running: boolean;
  /** Entries whose media properties are known. */
  done: number;
  total: number;
}

export function enrichStatus(): Promise<EnrichStatus> {
  return invoke<EnrichStatus>("enrich_status");
}

export interface DupeProgress {
  running: boolean;
  /** Files sharing a size, so needing a look. */
  candidates: number;
  hashed: number;
  groups: number;
  wasted: number;
}

export interface DupeGroup {
  size: number;
  /** Reclaimable by keeping one copy. */
  wasted: number;
  files: SearchHit[];
}

export function findDuplicates(): Promise<void> {
  return invoke("find_duplicates");
}

export function dupeProgress(): Promise<DupeProgress> {
  return invoke<DupeProgress>("dupe_progress");
}

export function dupeGroups(limit = 500): Promise<DupeGroup[]> {
  return invoke<DupeGroup[]>("dupe_groups", { limit });
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
  filters: Filters = NO_FILTERS,
  limit = 5000,
): Promise<SearchResponse | null> {
  const id = ++nextId;
  latestId = id;

  const res = await invoke<SearchResponse>("search", {
    id,
    query,
    kinds,
    limit,
    filters,
  });
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

export interface HotkeyStatus {
  /// `+`-separated, e.g. `Ctrl+Alt+Space`.
  combo: string;
  /// False when another application already owned the combination.
  active: boolean;
}

/// The backend owns the combination; asking keeps it from being written twice.
export function hotkeyStatus(): Promise<HotkeyStatus> {
  return invoke<HotkeyStatus>("hotkey_status");
}

export interface ScanProgress {
  phase: "volumes" | "scanning" | "resolving" | "indexing" | "saving" | "done" | "error";
  volume: string;
  records: number;
  mediaFiles: number;
  volumesDone: number;
  volumesTotal: number;
  message: string;
  /** Set only after the cache is safely on disk — see the indexer. */
  finished: boolean;
  error: string | null;
}

export interface ScanStatus {
  scanning: boolean;
  progress: ScanProgress | null;
}

/**
 * Start a rescan in an elevated child process.
 *
 * Rejects with a plain sentence if the user declines the UAC prompt; that is
 * an answer, not a failure, and the message says so.
 */
export function requestScan(): Promise<void> {
  return invoke("request_scan");
}

export function scanProgress(): Promise<ScanStatus> {
  return invoke<ScanStatus>("scan_progress");
}

/** Load the cache the indexer just wrote. */
export function reloadIndex(): Promise<IndexMeta> {
  return invoke<IndexMeta>("reload_index");
}

/**
 * URL of a result's thumbnail.
 *
 * `convertFileSrc` maps the custom scheme to whatever form the platform's
 * webview accepts — on Windows `thumb://` becomes `http://thumb.localhost`.
 * Writing the scheme by hand would work in a browser and silently 404 here.
 *
 * The epoch pins the URL to the index that produced it, so a rescan landing
 * mid-scroll cannot paint one file's picture beside another file's name.
 *
 * The two numbers are joined with `_` rather than `/`: `convertFileSrc`
 * percent-encodes what it is given, so a slash reaches the backend as `%2F`
 * and splits nothing.
 */
export function thumbUrl(epoch: number, index: number, size: number): string {
  return `${convertFileSrc(`${epoch}_${index}`, "thumb")}?s=${size}`;
}

export function formatCount(n: number): string {
  return n.toLocaleString("vi-VN");
}

/**
 * Bytes in the largest unit that keeps the number small.
 *
 * The first version stopped at MB, which was fine until the duplicate finder
 * reported a group as "17048.5 MB" and a total as "533214.6 MB". Both are
 * correct and neither is readable — nobody holds a six-digit megabyte figure
 * in their head. A media library deals in gigabytes and terabytes.
 */
export function formatBytes(n: number): string {
  const K = 1024;
  if (n < K) return `${n} B`;
  if (n < K ** 2) return `${(n / K).toFixed(1)} KB`;
  if (n < K ** 3) return `${(n / K ** 2).toFixed(1)} MB`;
  if (n < K ** 4) return `${(n / K ** 3).toFixed(1)} GB`;
  return `${(n / K ** 4).toFixed(2)} TB`;
}

/** `1:23` or `1:02:03` — the shape a media player uses. */
export function formatDuration(ms: number): string {
  if (!ms) return "";
  const total = Math.round(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/**
 * The label people actually use for a resolution.
 *
 * Keyed on the shorter side, so a 1080x1920 phone video reads as 1080p just
 * like a 1920x1080 one — which is what the person who shot it would call it.
 */
export function formatResolution(w: number, h: number): string {
  if (!w || !h) return "";
  const short = Math.min(w, h);
  if (short >= 2160) return "4K";
  if (short >= 1440) return "1440p";
  if (short >= 1080) return "1080p";
  if (short >= 720) return "720p";
  if (short >= 480) return "480p";
  return `${w}×${h}`;
}

export function formatWhen(unix: number): string {
  if (!unix) return "";
  return new Date(unix * 1000).toLocaleString("vi-VN");
}
