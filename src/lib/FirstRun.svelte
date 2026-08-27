<script lang="ts">
  import type { NetworkDrive } from "./search";

  let {
    netDrives,
    onscan,
  }: {
    netDrives: NetworkDrive[];
    onscan: (withNetwork: boolean) => void;
  } = $props();
</script>

<div class="firstrun">
  <div class="big">Chưa có chỉ mục — cần quét ổ đĩa một lần</div>
  <p>
    MediaFinder đọc thẳng bảng tệp của NTFS nên tìm ra kết quả trong vài
    mili giây. Đổi lại, nó phải đọc bảng đó một lần trước đã.
  </p>
  <ul>
    <li>
      Windows sẽ hỏi quyền Administrator <strong>đúng một lần</strong> —
      đọc bảng tệp bắt buộc phải có quyền đó.
    </li>
    <li>
      Lần quét đầu mất khoảng <strong>nửa phút tới vài phút</strong>, tuỳ
      số tệp trong máy.
    </li>
    <li>
      Sau lần này ứng dụng tự cập nhật lúc đăng nhập và mỗi ngày một lần,
      và <strong>không hỏi quyền lần nào nữa</strong>.
    </li>
    <li>
      Ứng dụng chạy sẵn cùng Windows ở chế độ ẩn để phím tắt
      <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Space</kbd> luôn gọi được — tắt
      phần này lúc nào cũng được, hướng dẫn nằm trong tệp đọc thêm.
    </li>
  </ul>
  <div class="actions">
    <button class="primary" onclick={() => onscan(false)}>Quét lần đầu</button>
    {#if netDrives.length}
      <button onclick={() => onscan(true)}>
        Quét cả ổ mạng ({netDrives.map((d) => d.letter).join(", ")})
      </button>
    {/if}
  </div>
  {#if netDrives.length}
    <p class="quiet">
      Ổ mạng lâu hơn nhiều lần. Bỏ qua bây giờ cũng được — nút
      <strong>+ ổ mạng</strong> ở trên làm đúng việc đó bất cứ lúc nào.
    </p>
  {/if}
</div>

<style>
  .firstrun {
    max-width: 560px;
    margin: 48px auto 0;
    padding: 0 24px;
    text-align: left;
    line-height: 1.6;
  }
  .firstrun .big {
    font-size: 17px;
    font-weight: 600;
    margin-bottom: 10px;
    text-align: center;
  }
  .firstrun p {
    color: var(--dim, #8b93a3);
    margin: 0 0 12px;
  }
  .firstrun ul {
    color: var(--dim, #8b93a3);
    margin: 0 0 20px;
    padding-left: 20px;
  }
  .firstrun li {
    margin-bottom: 6px;
  }
  .firstrun kbd {
    font: inherit;
    font-size: 11px;
    padding: 1px 5px;
    border: 1px solid var(--line, #2a2e37);
    border-radius: 4px;
  }
  .firstrun .actions {
    display: flex;
    gap: 10px;
    justify-content: center;
    flex-wrap: wrap;
  }
  .firstrun button {
    padding: 9px 18px;
    border-radius: 8px;
    border: 1px solid var(--line, #2a2e37);
    background: var(--hover, #232833);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .firstrun button.primary {
    background: #2d6cdf;
    border-color: #2d6cdf;
    color: #fff;
    font-weight: 600;
  }
  .firstrun button:hover {
    filter: brightness(1.12);
  }
  .firstrun .quiet {
    margin-top: 14px;
    font-size: 12px;
    text-align: center;
  }
</style>
