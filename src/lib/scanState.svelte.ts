import {
  reloadIndex,
  requestScan,
  requestScanWithNetwork,
  scanProgress,
  type IndexMeta,
  type ScanProgress,
} from "./search";

/// Vòng đời của một lần quét ổ đĩa, tách khỏi giao diện.
///
/// Đây là một lớp chứ không phải các biến ở tầng module: các biến tầng module
/// sẽ được chia sẻ giữa mọi thứ nhập tệp này, và "một lần quét" là thứ có bắt
/// đầu và kết thúc chứ không phải một sự thật vĩnh viễn của chương trình.
/// Trong một cửa sổ duy nhất thì hai cách chạy như nhau, nhưng cách này không
/// đặt sẵn cái bẫy cho cửa sổ thứ hai.
export class ScanState {
  scanning = $state(false);
  progress = $state<ScanProgress | null>(null);
  network = $state(false);

  #timer: ReturnType<typeof setInterval> | undefined;

  /// Được gọi khi một lần quét kết thúc êm đẹp và chỉ mục đã được đọc lại.
  /// Việc tìm kiếm cần biết để chạy lại, vì mọi số hiệu tệp đã đổi.
  #onreload: (meta: IndexMeta) => void;
  #onerror: (message: string) => void;

  constructor(opts: {
    onreload: (meta: IndexMeta) => void;
    onerror: (message: string) => void;
  }) {
    this.#onreload = opts.onreload;
    this.#onerror = opts.onerror;
  }

  /// Bắt kịp một lần quét đã chạy sẵn từ trước khi cửa sổ mở ra.
  async resume() {
    const s = await scanProgress();
    if (s.scanning) this.#poll();
  }

  async start(withNetwork = false) {
    try {
      await (withNetwork ? requestScanWithNetwork() : requestScan());
      this.network = withNetwork;
      this.progress = null;
      this.#poll();
    } catch (e) {
      // Từ chối UAC thì rơi vào đây. Backend đã diễn đạt nó như một câu trả
      // lời chứ không phải một lỗi, nên hiện nguyên văn.
      this.#onerror(String(e));
    }
  }

  #poll() {
    this.scanning = true;
    clearInterval(this.#timer);
    this.#timer = setInterval(async () => {
      let status;
      try {
        status = await scanProgress();
      } catch {
        return; // thoáng qua; nhịp sau bắt lại
      }
      this.progress = status.progress;

      // `finished` chỉ được bộ lập chỉ mục đặt sau khi cache đã ghi xong, nên
      // đọc lại ở đây không bao giờ gặp một tệp mới ghi được một nửa.
      if (status.progress?.finished) {
        this.#stop();
        if (status.progress.error) {
          this.#onerror(status.progress.error);
        } else {
          try {
            this.#onreload(await reloadIndex());
          } catch (e) {
            this.#onerror(String(e));
          }
        }
        return;
      }

      // Tiến trình con chết mà không báo gì — một lần sập, hoặc Windows từ
      // chối khởi động nó. Thiếu chỗ này thì thanh tiến trình quay mãi.
      if (!status.scanning) {
        this.#stop();
        if (!status.progress?.finished) {
          this.#onerror("Tiến trình quét kết thúc bất thường. Dữ liệu cũ vẫn nguyên.");
        }
      }
    }, 250);
  }

  #stop() {
    clearInterval(this.#timer);
    this.#timer = undefined;
    this.scanning = false;
    this.progress = null;
  }

  /// Gọi khi cửa sổ đóng lại, để nhịp hẹn giờ không sống lâu hơn giao diện.
  dispose() {
    clearInterval(this.#timer);
    this.#timer = undefined;
  }
}
