<script lang="ts">
  import { formatCount, type DupeScope, type ScopeEstimate } from "./search";

  let {
    est,
    idleOn,
    onchoose,
    oncancel,
    ontoggleidle,
  }: {
    est: ScopeEstimate;
    /// Quét nền ổ trong máy đang bật không.
    idleOn: boolean;
    onchoose: (scope: DupeScope) => void;
    oncancel: () => void;
    ontoggleidle: (enabled: boolean) => void;
  } = $props();

  const tongTep = $derived(est.localFiles + est.networkFiles);
  const dsOMang = $derived(est.networkDrives.join(", "));

  /// Đọc số tệp thành một câu về công sức, không phải một lời hứa về phút.
  ///
  /// KHÔNG hiện số phút ở đây, cố ý. Không có phép đo nào cho mã hiện tại trên
  /// thư viện hiện tại, và tốc độ đọc NAS đổi theo giờ trong ngày cùng số máy
  /// đang cùng dùng. Hứa "khoảng 30 phút" rồi chạy 8 phút hoặc 50 phút thì lần
  /// sau không ai tin con số nào nữa.
  ///
  /// Thay vào đó nói thứ **đếm được** — bao nhiêu tệp phải mở — và để phần
  /// thời gian cho lúc quét đang chạy, khi đã có tốc độ thật của máy đó.
  function moTa(n: number, mang: boolean): string {
    if (n === 0) return "không có tệp nào cần kiểm";
    const tep = `${formatCount(n)} tệp`;
    return mang ? `${tep} — đọc qua mạng nên chậm hơn nhiều` : `${tep} — đọc từ đĩa trong máy`;
  }
</script>

<!--
  Hỏi trước khi quét ổ mạng.

  Cái giá của việc quét NAS không đổ lên máy người bấm nút — nó đổ lên chính
  NAS mà cả studio đang dùng để làm việc, và 20–40 máy cùng quét là 20–40 luồng
  đọc ngẫu nhiên trên cùng vài ổ đĩa. Người bấm phải biết mình đang chọn gì.

  Không đặt mặc định im lặng theo hướng nào: chọn sẵn "có" thì người chỉ muốn
  dọn ổ C: phải chờ NAS; chọn sẵn "không" thì người muốn dọn NAS tưởng app bỏ
  sót tệp.
-->
<div
  class="lop"
  role="dialog"
  aria-modal="true"
  aria-labelledby="dsa-tieu-de"
  tabindex="-1"
  onkeydown={(e) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      oncancel();
    }
  }}
>
  <div class="hop">
    <h2 id="dsa-tieu-de">Tìm tệp trùng lặp ở đâu?</h2>
    <p class="dan">
      Cần mở từng tệp có cùng dung lượng để đối chiếu nội dung — {formatCount(tongTep)} tệp nếu
      quét cả ổ mạng.
    </p>

    <div class="chon">
      <button class="lc" onclick={() => onchoose("localOnly")}>
        <span class="ten">Chỉ ổ trong máy</span>
        <span class="phu">{moTa(est.localFiles, false)}</span>
      </button>

      <button class="lc" onclick={() => onchoose("everything")}>
        <span class="ten">Cả ổ mạng {dsOMang}</span>
        <span class="phu">
          thêm {moTa(est.networkFiles, true)}
        </span>
      </button>
    </div>

    <p class="luu-y">
      Ổ mạng chậm hơn nhiều lần, và nhiều máy cùng quét sẽ làm NAS chậm đi với
      mọi người. Thời gian còn lại sẽ hiện ngay khi quét bắt đầu.
    </p>

    <!--
      Ô tắt quét nền, đặt ngay đây vì đây là lúc người dùng đang nghĩ về việc
      quét — chứ không phải trong một màn cài đặt mà họ sẽ không bao giờ mở.

      Nói rõ nó CHỈ quét ổ trong máy: người đọc dòng này vừa mới thấy con số
      "đọc qua mạng nên chậm hơn nhiều" ở trên, nên phải chặn ngay cách hiểu
      rằng máy đang âm thầm đọc NAS.
    -->
    <label class="nen">
      <input
        type="checkbox"
        checked={idleOn}
        onchange={(e) => ontoggleidle(e.currentTarget.checked)}
      />
      <span>
        Tự quét sẵn <b>ổ trong máy</b> lúc máy rảnh, để lần sau mở ra là có kết
        quả ngay. Không bao giờ tự đọc ổ mạng.
      </span>
    </label>

    <!--
      "Huỷ" chứ không phải "Để sau": người bấm nút này muốn thoát hẳn khỏi việc
      tìm trùng lặp, và màn hình phải về đúng trạng thái trước khi họ bấm — kể
      cả nút Trùng lặp trên thanh công cụ cũng tắt sáng. "Để sau" gợi ý một
      việc còn treo đó, mà không có việc nào treo cả.
    -->
    <button class="huy" onclick={oncancel}>Huỷ</button>
  </div>
</div>

<style>
  .lop {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    z-index: 20;
  }
  .hop {
    width: min(440px, 92%);
    padding: 22px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 17px;
    color: var(--text);
  }
  .dan {
    margin: 0 0 16px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-dim);
  }
  .chon {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .lc {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 11px 14px;
    text-align: left;
    font-family: inherit;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
  }
  .lc:hover {
    border-color: var(--accent);
  }
  .lc:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .ten {
    font-size: 14px;
    color: var(--text);
  }
  .phu {
    font-size: 12px;
    color: var(--text-dim);
  }
  .luu-y {
    margin: 14px 0 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-dim);
    opacity: 0.85;
  }
  .nen {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--text-dim);
    cursor: pointer;
  }
  .nen input {
    margin-top: 2px;
    flex: 0 0 auto;
    cursor: pointer;
  }
  .nen b {
    color: var(--text);
    font-weight: 500;
  }

  .huy {
    margin-top: 14px;
    padding: 5px 12px;
    font-family: inherit;
    font-size: 12.5px;
    color: var(--text-dim);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }
  .huy:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
