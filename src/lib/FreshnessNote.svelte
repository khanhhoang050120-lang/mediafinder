<script lang="ts">
  import type { LastCheck, NetScanMark, TaskHealth } from "./search";
  import {
    daCu,
    moTaTuoi,
    NGUONG_CUC_BO_GIAY,
    NGUONG_O_MANG_GIAY,
  } from "./freshness";

  let {
    builtAtUnix,
    netMark,
    health,
    check = null,
    hasNetDrives = true,
  }: {
    /// Mốc ghi cache gần nhất — tuổi của phần **ổ trong máy**.
    builtAtUnix: number;
    /// Lần quét ổ mạng gần nhất; `null` là chưa từng quét xong.
    netMark: NetScanMark | null;
    /// Tác vụ định kỳ còn sống không; `null` là chưa hỏi được.
    health: TaskHealth | null;
    /// Lượt kiểm gần nhất của tiến trình làm mới. Đây mới là con số trả lời
    /// đúng câu "chỉ mục còn được trông nom không" — `builtAtUnix` chỉ nói
    /// "chỉ mục đổi lần cuối lúc nào", mà bản vá gia tăng cố ý không ghi lại
    /// cache khi không có gì đổi.
    check?: LastCheck | null;
    /// Có ổ mạng nào đang gắn không. Không có thì đừng nhắc tới ổ mạng.
    hasNetDrives?: boolean;
  } = $props();

  /// Tuổi "thật" của ổ cục bộ = lần **kiểm** gần nhất, không phải lần **đổi**.
  ///
  /// Thiếu phân biệt này thì một máy hoàn toàn khoẻ — tác vụ vừa chạy hai phút
  /// trước — vẫn bị tô vàng "4 giờ trước" chỉ vì buổi tối không ai đụng vào tệp
  /// nào. Cảnh báo sai đúng lúc người dùng đang cố hiểu vì sao không ra kết quả
  /// còn tệ hơn không cảnh báo: nó chỉ họ đi sai hướng.
  ///
  /// Lùi về `builtAtUnix` khi chưa có mốc kiểm — máy vừa nâng cấp từ bản chưa
  /// có `lastcheck.json` sẽ ở trạng thái đó cho tới lượt tác vụ kế tiếp.
  const mocCucBo = $derived(check?.atUnix || builtAtUnix);
  const cucBoCu = $derived(daCu(mocCucBo, NGUONG_CUC_BO_GIAY));
  const mangCu = $derived(daCu(netMark?.atUnix ?? 0, NGUONG_O_MANG_GIAY));
  /// `netMark === null` **không** có nghĩa là "chưa quét lần nào".
  ///
  /// `netscan.json` là tệp mới của bản này, nên trên MỌI máy nâng cấp lên nó sẽ
  /// vắng mặt — kể cả máy đang có 320.505 mục ổ mạng trong chỉ mục. Nói "chưa
  /// quét lần nào" ở đó là nói sai, và nói sai đúng trên màn hình được dựng lên
  /// để "thôi để họ kết luận sai".
  ///
  /// Không có đường di trú nào khả dĩ: chỉ mục cố ý không cấp `VolumeStamp` cho
  /// ổ mạng, nên không suy ngược ra mốc cũ được. Nên sửa bằng CÂU CHỮ: chưa
  /// biết thì nói là chưa biết.
  const chuaRoMang = $derived(netMark === null);
  const matTacVu = $derived(health !== null && !health.taskExists);

  /// Chỉ lên tiếng khi có gì đó thật sự đáng nói. Một dòng chú thích hiện ở
  /// mọi lượt tìm kiếm sẽ bị mắt bỏ qua sau ngày thứ hai, và lúc nó có tin
  /// quan trọng thì không ai còn đọc nữa.
  // Không gắn ổ mạng nào thì chuyện ổ mạng cũ hay mới đều không liên quan.
  const noiVeMang = $derived(hasNetDrives !== false);
  const coGiDeNoi = $derived(cucBoCu || matTacVu || (noiVeMang && (mangCu || chuaRoMang)));
</script>

<!--
  Vì sao dòng này tồn tại.

  Người dùng gõ đúng tên một tệp có thật, không thấy nó, rồi nhìn xuống chân
  cửa sổ thấy "quét lúc 16:15" và kết luận phần mềm hỏng. Thật ra tệp của họ
  nằm trên ổ mạng, mà ổ mạng lần cuối được quét lúc 11:23 — chân cửa sổ chỉ
  biết mốc ghi cache, và mốc đó được đóng dấu lại ở mọi lần ghi, kể cả lượt vá
  gia tăng ổ cục bộ.

  Nên chỗ này nói hai mốc tách bạch, và nói ngay tại điểm hỏng: cả khi bộ tìm
  phải nới lỏng, lẫn khi không ra kết quả nào. Mục tiêu không phải "thêm thông
  tin" mà là **thôi để họ kết luận sai**.
-->
{#if coGiDeNoi}
  <div class="fresh" role="note">
    {#if matTacVu}
      <span class="canh">
        Không còn tác vụ làm mới định kỳ trên máy này — chỉ mục sẽ không tự cập
        nhật nữa. Bấm <b>Quét lại</b> một lần để tạo lại nó.
      </span>
    {/if}

    <span class="moc">
      Ổ trong máy: <b class:cu={cucBoCu}>{moTaTuoi(mocCucBo) || "chưa rõ"}</b>
      {#if noiVeMang}
        <span class="ngan">·</span>
        Ổ mạng:
        {#if chuaRoMang}
          <b>chưa rõ lần trước</b> — bấm <b>+ ổ mạng</b> để làm mới
        {:else}
          <b class:cu={mangCu}>{moTaTuoi(netMark!.atUnix)}</b>
        {/if}
      {/if}
    </span>
  </div>
{/if}

<style>
  .fresh {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 6px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted, #8b93a7);
  }
  .canh {
    color: var(--warn, #e0b341);
  }
  .moc b {
    font-weight: 600;
    color: var(--fg, #d6dae4);
  }
  /* Chỉ tô cái đang cũ. Tô cả hai thì không còn phân biệt được cái nào là
     vấn đề, mà thường chỉ một trong hai mới là. */
  .moc b.cu {
    color: var(--warn, #e0b341);
  }
  .ngan {
    opacity: 0.5;
    margin: 0 4px;
  }
</style>
