import { previewClose, previewOpen, previewRead, type PreviewOpen } from "./search";

/// Phát một video đã chuyển mã bằng Media Source: nạp từng mảnh ngay khi về.
///
/// # Vì sao không đơn giản là `<video src>`
///
/// Codec như Apple ProRes thì WebView2 không giải mã được, nên backend chuyển
/// mã sang H.264 trong lúc người dùng xem. Qua `src` thì trình phát chỉ nhận
/// được video khi đã chuyển mã **xong** — bản trước bắt người dùng chờ, rồi để
/// bớt chờ lại cắt video còn 5 giây, và video dừng hẳn ở giây thứ 5.
///
/// Media Source thì khác: mảnh đầu về (~0,4 giây) là có hình, các mảnh sau nối
/// vào trong lúc đang xem, và thời lượng đặt ngay từ đầu nên thanh thời gian
/// hiện đủ độ dài thật.
///
/// # Tua tới chỗ chưa có
///
/// Mỗi phiên backend chuyển mã từ một mốc tới hết tệp. Người dùng kéo tới một
/// chỗ chưa về — và phiên đang chạy sẽ không tới đó sớm — thì đóng phiên cũ,
/// mở phiên mới từ đúng chỗ ấy, và dời dữ liệu của nó về đúng vị trí bằng
/// `timestampOffset`. Phần đã nạp vẫn giữ nguyên, nên quay lại chỗ cũ là tức
/// thì.
export class TranscodePlayer {
  readonly #video: HTMLVideoElement;
  readonly #epoch: number;
  readonly #index: number;
  readonly #force: boolean;
  readonly #onFatal: (message: string) => void;
  readonly #mime: string;
  readonly #duration: number;

  #ms: MediaSource | null = null;
  #url = "";
  #sb: SourceBuffer | null = null;

  /// Phiên backend đang nạp, và mốc (giây trong tệp gốc) nó bắt đầu.
  #session: number;
  #from: number;
  /// Phiên đang nạp đã tới cuối tệp — nó sẽ không mang thêm gì.
  #activeDone = false;
  /// Tăng mỗi khi đổi phiên. Vòng nạp nào thấy số này đổi thì tự bỏ: dữ
  /// liệu của nó thuộc về một mốc không còn ai cần.
  #gen = 0;
  #stopped = false;

  constructor(
    video: HTMLVideoElement,
    epoch: number,
    index: number,
    first: PreviewOpen,
    force: boolean,
    onFatal: (message: string) => void,
  ) {
    this.#video = video;
    this.#epoch = epoch;
    this.#index = index;
    this.#force = force;
    this.#onFatal = onFatal;
    this.#mime = first.mime;
    this.#duration = first.duration;
    this.#session = first.session;
    this.#from = first.from;
  }

  start(): void {
    const ms = new MediaSource();
    this.#ms = ms;
    this.#url = URL.createObjectURL(ms);
    ms.addEventListener("sourceopen", this.#onOpen, { once: true });
    this.#video.addEventListener("seeking", this.#onSeekOrWait);
    this.#video.addEventListener("waiting", this.#onSeekOrWait);
    this.#video.src = this.#url;
  }

  /// Dừng hẳn. Gọi được nhiều lần.
  destroy(): void {
    if (this.#stopped) return;
    this.#stopped = true;
    this.#gen++;
    this.#video.removeEventListener("seeking", this.#onSeekOrWait);
    this.#video.removeEventListener("waiting", this.#onSeekOrWait);
    previewClose(this.#session).catch(() => {});
    if (this.#url) URL.revokeObjectURL(this.#url);
  }

  #onOpen = () => {
    if (this.#stopped || !this.#ms) return;
    try {
      // Đặt TRƯỚC khi nạp gì: thanh thời gian hiện đủ độ dài thật ngay từ
      // khung hình đầu, không phải chỉ phần đã về.
      this.#ms.duration = this.#duration;
      this.#sb = this.#ms.addSourceBuffer(this.#mime);
      this.#sb.mode = "segments";
      this.#sb.timestampOffset = this.#from;
    } catch (e) {
      this.#fatal(e);
      return;
    }
    void this.#pump(this.#gen, this.#session);
  };

  /// Kéo dữ liệu của một phiên về và nạp vào trình phát, cho tới hết.
  async #pump(gen: number, session: number): Promise<void> {
    let offset = 0;
    while (this.#current(gen)) {
      let chunk;
      try {
        chunk = await previewRead(session, offset);
      } catch (e) {
        if (this.#current(gen)) this.#fatal(e);
        return;
      }
      if (!this.#current(gen)) return;
      if (chunk.flag === 2) {
        this.#fatal("phiên chuyển mã không còn");
        return;
      }
      if (chunk.data.length) {
        offset += chunk.data.length;
        if (!(await this.#append(chunk.data, gen))) return;
      }
      if (chunk.flag === 1) {
        this.#activeDone = true;
        await this.#finish(gen);
        return;
      }
    }
  }

  async #append(data: Uint8Array, gen: number): Promise<boolean> {
    const sb = this.#sb;
    if (!sb) return false;
    while (this.#current(gen)) {
      await this.#idle();
      if (!this.#current(gen)) return false;
      try {
        sb.appendBuffer(data as BufferSource);
        await this.#idle();
        return this.#current(gen);
      } catch (e) {
        if ((e as DOMException)?.name !== "QuotaExceededError") {
          this.#fatal(e);
          return false;
        }
        // Bộ đệm của trình phát đầy — chỉ xảy ra với video dài. Bỏ bớt
        // phần đã xem xa phía sau; không có gì để bỏ thì chờ người xem đi
        // tiếp một chút.
        const cut = this.#video.currentTime - 10;
        if (cut > 1) {
          try {
            sb.remove(0, cut);
            await this.#idle();
          } catch {
            /* thử lại vòng sau */
          }
        } else {
          await new Promise((r) => setTimeout(r, 1000));
        }
      }
    }
    return false;
  }

  /// Phiên đã tới cuối tệp: báo hết luồng, để tới cuối là `ended` chứ không
  /// đứng chờ mãi một mảnh không bao giờ tới.
  async #finish(gen: number): Promise<void> {
    await this.#idle();
    if (!this.#current(gen) || this.#ms?.readyState !== "open") return;
    try {
      this.#ms.endOfStream();
    } catch {
      /* đang nạp dở ở phiên khác — không sao */
    }
  }

  #onSeekOrWait = () => {
    if (this.#stopped || !this.#sb) return;
    const t = this.#video.currentTime;
    if (this.#isBuffered(t) || this.#activeCovers(t)) return;
    void this.#switchTo(t);
  };

  /// `t` đã nằm trong phần nạp rồi.
  #isBuffered(t: number): boolean {
    const b = this.#sb!.buffered;
    for (let i = 0; i < b.length; i++) {
      if (t >= b.start(i) - 0.05 && t < b.end(i) - 0.1) return true;
    }
    return false;
  }

  /// Phiên đang chạy sắp tới `t` — chờ nó rẻ hơn mở phiên mới.
  ///
  /// Hai giây: chuyển mã chạy nhanh hơn thời gian thực 2–4 lần, nên hai giây
  /// phía trước tới trong chưa tới một giây — ngang với giá mở một phiên mới,
  /// mà không vứt bỏ phần phiên cũ đang làm dở.
  #activeCovers(t: number): boolean {
    if (this.#activeDone || t < this.#from - 0.05) return false;
    return t <= this.#rangeEndFrom(this.#from) + 2;
  }

  /// Điểm cuối của vùng đã nạp liền mạch tính từ `x`.
  #rangeEndFrom(x: number): number {
    const b = this.#sb!.buffered;
    for (let i = 0; i < b.length; i++) {
      if (x >= b.start(i) - 0.3 && x <= b.end(i)) return b.end(i);
    }
    return x;
  }

  /// Bỏ phiên đang chạy, mở phiên mới từ `t`.
  async #switchTo(t: number): Promise<void> {
    const gen = ++this.#gen;
    const old = this.#session;
    // Ghi mốc mới NGAY, trước khi chờ gì: `waiting` bắn liên tục trong lúc
    // mở phiên, và nếu mốc còn là của phiên cũ thì mỗi lần bắn lại mở thêm
    // một phiên nữa.
    this.#from = t;
    this.#activeDone = false;
    previewClose(old).catch(() => {});

    let info: PreviewOpen;
    try {
      info = await previewOpen(this.#epoch, this.#index, t, this.#force);
    } catch (e) {
      if (this.#current(gen)) this.#fatal(e);
      return;
    }
    if (!this.#current(gen) || info.kind !== "stream") {
      if (info.kind === "stream") previewClose(info.session).catch(() => {});
      return;
    }
    await this.#idle();
    if (!this.#current(gen)) {
      previewClose(info.session).catch(() => {});
      return;
    }
    try {
      // Bỏ dở phần đang phân tích của phiên cũ: mảnh cuối của nó có thể
      // mới về một nửa, và nối nửa mảnh với đầu phiên mới là dữ liệu rác.
      if (this.#ms?.readyState === "open") this.#sb!.abort();
      // Phiên mới bắt đầu từ mốc 0; dời nó về đúng chỗ trong video.
      this.#sb!.timestampOffset = info.from;
    } catch (e) {
      this.#fatal(e);
      return;
    }
    this.#session = info.session;
    this.#from = info.from;
    void this.#pump(gen, info.session);
  }

  #idle(): Promise<void> {
    const sb = this.#sb;
    if (!sb || !sb.updating) return Promise.resolve();
    return new Promise((resolve) => {
      const done = () => {
        sb.removeEventListener("updateend", done);
        sb.removeEventListener("error", done);
        sb.removeEventListener("abort", done);
        resolve();
      };
      sb.addEventListener("updateend", done);
      sb.addEventListener("error", done);
      sb.addEventListener("abort", done);
    });
  }

  #current(gen: number): boolean {
    return !this.#stopped && gen === this.#gen;
  }

  #fatal(e: unknown): void {
    if (this.#stopped) return;
    this.#onFatal(String(e));
  }
}
