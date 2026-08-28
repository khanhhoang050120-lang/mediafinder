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
  /**
   * Only files modified within this many days; 0 disables.
   *
   * Days rather than a timestamp, so the window stays relative to now — a
   * window left open overnight should still mean "the last seven days" in the
   * morning, not "the seven days ending yesterday evening".
   */
  withinDays: number;
}

export const NO_FILTERS: Filters = {
  minHeight: 0,
  minDurationMs: 0,
  maxDurationMs: 0,
  withinDays: 0,
};

/** What the result list is ordered by. Mirrors `search::Order` in Rust. */
export type Order = "relevance" | "newest";

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
  /**
   * A scan has finished and `groups` is its answer.
   *
   * Not the same as `groups > 0`: a library with nothing duplicated is a
   * finished scan with an empty answer, and treating that as "never scanned"
   * would re-run ten minutes of disk reading on every visit.
   */
  completed: boolean;
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

/**
 * Stop a scan the person has walked away from.
 *
 * The scan reads the disk for minutes; leaving it running would compete with
 * whatever they went back to doing.
 */
export function cancelDuplicates(): Promise<void> {
  return invoke("cancel_duplicates");
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
  order: Order = "relevance",
): Promise<SearchResponse | null> {
  const id = ++nextId;
  latestId = id;

  // Grouped into one `req` because the backend takes it that way: the argument
  // list had grown to eight and every addition made it harder to read. `id`
  // stays outside — it is not part of the question, it is how a superseded
  // answer gets recognised.
  const res = await invoke<SearchResponse>("search", {
    id,
    req: { query, kinds, limit, filters, order },
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

export interface NetworkDrive {
  letter: string;
  remote: string;
}

/// Mapped network drives, so the UI can name them instead of saying "NAS".
export function networkDrives(): Promise<NetworkDrive[]> {
  return invoke<NetworkDrive[]>("network_drives");
}

/// Scan the local disks and then walk every network drive.
///
/// Separate from `requestScan` on purpose: this one takes minutes rather than
/// seconds, so it must never happen unless the user asked for it.
export function requestScanWithNetwork(): Promise<void> {
  return invoke("request_scan_with_network");
}

/// Ask the running scan to stop. Only the network phase can honour it.
export function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

/// Start dragging files out of the window.
///
/// Must be called from a `dragstart` handler that has already called
/// `preventDefault()`. The WebView's own drag offers web formats — text, a URL
/// — and nothing that takes files will look at those; the native drag offers
/// `CF_HDROP`, which is what CapCut, Explorer and upload fields read.
export function startFileDrag(paths: string[]): Promise<void> {
  return invoke("start_file_drag", { paths });
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
  /// Must match the phases `ScanProgress` in `ipc/elevate.rs` writes. The two
  /// sides are one contract; adding a phase on one only is how a UI ends up
  /// silently never showing it.
  phase:
    | "volumes"
    | "scanning"
    | "resolving"
    | "indexing"
    | "network"
    | "saving"
    | "done"
    | "error";
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

/**
 * URL of a result's own bytes, for playing it inside the app.
 *
 * Same shape and same epoch rule as {@link thumbUrl}: the page names a
 * position in the index, never a path. A URL that could carry a path would let
 * the page read any file on the machine; this one can only reach files the
 * index already holds.
 */
export function mediaUrl(epoch: number, index: number): string {
  return convertFileSrc(`${epoch}_${index}`, "media");
}

/** Bản mới máy chủ đang mời, kèm ghi chú "có gì mới" nếu có. */
export interface UpdateFound {
  version: string;
  /**
   * Nội dung release notes từ latest.json. Ghi chú mô tả *bản mới*, nên chỉ
   * máy chủ mới biết — app đang chạy không thể tự bịa ra danh sách tính năng
   * của một bản nó chưa từng thấy.
   */
  notes: string | null;
}

export interface UpdateStatus {
  /** Đã hỏi máy chủ xong chưa. `false` nghĩa là chưa biết, không phải là không có bản mới. */
  checked: boolean;
  /** Bản mới nếu có. */
  available: UpdateFound | null;
  /** Phiên bản đang chạy. */
  current: string;
}

/**
 * Tình hình cập nhật mà lần kiểm tra lúc khởi động đã tìm ra.
 *
 * Chỉ đọc thứ backend ghi sẵn — không gọi mạng, nên gọi bao nhiêu lần cũng rẻ.
 */
export function updateStatus(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("update_status");
}

/**
 * Tải bản mới về và cài, rồi khởi động lại.
 *
 * Chỉ gọi khi người dùng đã đồng ý: bộ cài hơn 200 MB và đây là đường truyền
 * của họ. Trong lúc tải, `onProgress` cho biết đã được bao nhiêu phần trăm —
 * không có nó thì màn hình đứng im hàng phút và trông như bị treo.
 */
/** Lần quét ổ mạng gần nhất đã hoàn tất; `null` khi chưa từng quét xong. */
export interface NetScanMark {
  atUnix: number;
  files: number;
  drives: number;
  seconds: number;
}

export function netScanMark(): Promise<NetScanMark | null> {
  return invoke<NetScanMark | null>("net_scan_mark");
}

/** Cỗ máy làm mới chỉ mục còn sống hay không. */
export interface TaskHealth {
  taskExists: boolean;
}

/**
 * Hỏi xem tác vụ định kỳ còn trên máy không.
 *
 * Gọi thưa: mỗi lượt sinh một tiến trình `schtasks.exe`. Chỗ đúng là lúc mở
 * cửa sổ và sau mỗi lượt quét, không phải mỗi lần gõ phím.
 */
export function taskHealth(): Promise<TaskHealth | null> {
  return invoke<TaskHealth | null>("task_health");
}

/// Lượt kiểm tra gần nhất của tiến trình làm mới.
export interface LastCheck {
  atUnix: number;
  changed: boolean;
}

/**
 * "Cỗ máy làm mới chạy lần cuối lúc nào" — khác với "chỉ mục đổi lần cuối lúc
 * nào".
 *
 * Bản vá gia tăng cố ý không ghi lại cache khi không có gì đổi, nên
 * `builtAtUnix` đứng yên trên một máy hoàn toàn khoẻ chỉ vì buổi tối không ai
 * đụng vào tệp nào. Đây là con số trả lời đúng câu "chỉ mục còn được trông
 * nom không".
 */
export function lastCheck(): Promise<LastCheck | null> {
  return invoke<LastCheck | null>("last_check");
}

/** Tình hình bộ ghi truy-vấn-0-kết-quả (đo chất lượng tìm kiếm, cục bộ). */
export interface MissLogStatus {
  enabled: boolean;
  count: number;
}

export function missLogStatus(): Promise<MissLogStatus> {
  return invoke<MissLogStatus>("miss_log_status");
}

export function missLogSetEnabled(enabled: boolean): Promise<void> {
  return invoke("miss_log_set_enabled", { enabled });
}

export function missLogClear(): Promise<void> {
  return invoke("miss_log_clear");
}

/** Mở file ghi bằng trình soạn thảo mặc định. */
export function missLogOpen(): Promise<void> {
  return invoke("miss_log_open");
}

/**
 * Tầng 3 tìm-trùng: xác minh trọn nội dung một nhóm ứng viên.
 *
 * `groups` là các cụm trùng thật sự từng byte — một cụm duy nhất nghĩa là
 * nhóm đúng là bản sao của nhau; `unreadable` là tệp không đọc nổi, về chúng
 * ta không kết luận gì.
 */
export interface VerifyOutcome {
  groups: string[][];
  unreadable: string[];
}

export function verifyDupeGroup(paths: string[]): Promise<VerifyOutcome> {
  return invoke<VerifyOutcome>("verify_dupe_group", { paths });
}

/** Mở trang Releases trên trình duyệt — cho ai muốn đọc nhiều hơn tóm tắt. */
export function openReleasesPage(): Promise<void> {
  return invoke("open_releases_page");
}

export async function installUpdate(
  onProgress: (percent: number) => void,
): Promise<void> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const { relaunch } = await import("@tauri-apps/plugin-process");

  const update = await check();
  if (!update) return;

  let total = 0;
  let got = 0;
  await update.downloadAndInstall((e) => {
    if (e.event === "Started") {
      total = e.data.contentLength ?? 0;
    } else if (e.event === "Progress") {
      got += e.data.chunkLength;
      // Máy chủ không phải lúc nào cũng nói trước tổng dung lượng; khi
      // không biết thì thà không hiện phần trăm còn hơn hiện số sai.
      if (total > 0) onProgress(Math.round((got / total) * 100));
    }
  });

  await relaunch();
}
