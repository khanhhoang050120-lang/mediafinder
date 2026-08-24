# MediaFinder

Công cụ tìm kiếm file media (video / ảnh / nhạc) tức thời trên Windows, đọc trực tiếp
NTFS MFT thay vì duyệt cây thư mục.

Rust + Tauri v2 + Svelte 5.

## Chạy dev

```bash
npm install
npm run tauri dev
```

Lần chạy đầu cần quét ổ đĩa → sẽ có một prompt UAC (chỉ cho tiến trình con `--index`).
Các lần sau load từ cache, **không có UAC**.

## Tài liệu

| File | Nội dung |
|---|---|
| [PROGRESS.md](./PROGRESS.md) | Tiến độ 9 giai đoạn, tiêu chí nghiệm thu, nhật ký kiểm chứng |
| [docs/](./docs/) | Sổ ghi vấn đề — lỗi, cấu hình, rủi ro, hiệu năng, kiểm chứng, sản phẩm, đặc tả, nhật ký test |

## Bất biến kiến trúc

Những điểm dưới đây là **cố ý**. Đọc trước khi định "sửa cho gọn" — mỗi cái đều
đã có lý do cụ thể.

1. **Quét là 2 pha, không phải 1.** Record MFT/USN chỉ mang `ParentFileReferenceNumber`,
   không mang đường dẫn. Không thể biết một file có nằm dưới `C:\Windows` hay không cho
   tới khi *toàn bộ* record thư mục đã đọc xong. Pha 1 chỉ lọc được **phần mở rộng**;
   lọc theo thư mục bắt buộc phải ở pha 2.

2. **`FSCTL_ENUM_USN_DATA`, không parse `$MFT` thô.** Windows đã bóc tách sẵn record.
   Parse thô buộc phải tự xử lý fixup array, runlist, `$ATTRIBUTE_LIST`, và — dễ sót
   nhất — **lọc namespace tên DOS 8.3**, nếu không mỗi file sẽ xuất hiện hai lần.

3. **GUI không bao giờ chạy elevated.** Manifest là `asInvoker` (xem `src-tauri/build.rs`).
   Chỉ tiến trình con `--index` mới elevate qua `ShellExecuteW(verb="runas")`.
   Lý do: UIPI chặn kéo-thả từ Explorer vào tiến trình elevated, và UAC mỗi lần mở app
   là không chấp nhận được.

4. **Không debounce.** Backend tìm ~5–20ms; debounce 200ms sẽ chiếm 90% độ trễ người dùng
   cảm nhận. Thay bằng coalesce 30ms + generation counter huỷ request cũ.

5. **Kết quả phải được xếp hạng trước khi cắt.** "Lấy N cái đầu tiên" theo thứ tự MFT là
   ngẫu nhiên. Ngoài ra `par_iter().filter().take(n)` không compile (`take` cần
   `IndexedParallelIterator`), còn `take_any` cho thứ tự không xác định → kết quả nhảy
   loạn giữa các lần tìm cùng từ khoá.

6. **Tìm kiếm chạy trên tên đã fold, không phải tên gốc.** Fold =
   NFD → bỏ combining marks → `đ`/`Đ` → `d` → lowercase.
   ⚠️ `ơ ư` **có** phân rã dưới NFD nên tự động xử lý; `đ` (U+0111) **không** phân rã,
   bắt buộc map tay. Thiếu bước này thì `tieng viet` không khớp `Tiếng Việt.mp4`.

7. **Search không giữ lock.** `ArcSwap<Index>` — clone `Arc` rồi nhả ngay, quá trình tìm
   song song chạy trên snapshot bất biến. Không bao giờ giữ `MutexGuard` xuyên qua `.await`
   trong Tauri command.

8. **Thumbnail đi qua `thumb://`, không qua IPC JSON.** WebView tự lazy-load, tự cache,
   tự tải song song. Nhét base64 vào response tìm kiếm sẽ tạo payload hàng trăm KB mỗi
   lần gõ phím.

9. **Chỉ hỗ trợ NTFS.** Volume exFAT/FAT32 (USB, thẻ SD) không có MFT/USN — được phát hiện
   và báo rõ cho người dùng, không im lặng bỏ qua.

## Bố cục

```
src/                  Svelte 5 + TypeScript (chỉ hiển thị, không lọc dữ liệu)
src-tauri/src/
  ntfs/               Truy cập volume, liệt kê USN (pha 1), dựng cây + resolve path (pha 2)
  index/              Model + fold + search + cache trên đĩa
  media/              Thumbnail, enrichment metadata, tìm file trùng
  ipc/                Tauri command, protocol thumb://, luồng elevate
  state.rs            ArcSwap<Index> + generation counter
```

`ntfs/tree.rs` cố ý **không** chứa type Win32 nào — nó nhận `&[RawRecord]` thuần. Đây là
seam giúp test logic dựng cây/resolve path trên CI mà không cần ổ NTFS thật hay quyền Admin.
