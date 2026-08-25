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

### Nếu gặp `cargo ... program not found`

PATH của terminal đã cũ, không phải Rust chưa cài. Trên Windows, một tiến trình giữ bản sao biến
môi trường từ lúc nó khởi động và không bao giờ thấy thay đổi sau đó.

**Khởi động lại VS Code** — mở tab terminal mới là *không đủ*, vì tab mới kế thừa môi trường từ
chính tiến trình VS Code. Hoặc tạm thời cho một cửa sổ:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Chi tiết: [`docs/config.md`](./docs/config.md#conf-003)

## Dùng hằng ngày

| Việc | Cách |
|---|---|
| Mở phần mềm | `Ctrl+Alt+Space` từ bất kỳ đâu, Start Menu, hoặc bấm biểu tượng ở khay |
| Ẩn đi | bấm lại đúng phím đó, hoặc đóng cửa sổ |
| **Tắt hẳn** | chuột phải biểu tượng ở khay → **Thoát** |
| Cập nhật ổ trong máy | tự động lúc đăng nhập (~0,45 s, không UAC), hoặc nút **Quét lại** |
| Cập nhật ổ mạng / NAS | nút **+ ổ mạng** — vài phút, chỉ khi bạn bấm |

Ứng dụng khởi động cùng Windows ở chế độ **ẩn**: nó đăng ký phím tắt rồi chờ, không mở cửa sổ nào.
Phím tắt chỉ hoạt động khi ứng dụng đang chạy, nên đây là điều kiện để nó dùng được.

Vì cùng lý do đó, **đóng cửa sổ chỉ ẩn đi**. Biểu tượng ở khay hệ thống là dấu hiệu cho biết nó vẫn
ở đó, và menu chuột phải là cách tắt hẳn. Tắt máy thì nó tắt theo như mọi chương trình khác — đã
kiểm chứng là không cản trở quá trình tắt máy.

Tự cập nhật chạy qua một Scheduled Task với quyền cao — đó là cách duy nhất đọc được USN journal
mà **không** hiện UAC mỗi lần đăng nhập ([CHECK-004](./docs/check.md#check-004)). Tác vụ này không
đụng tới ổ mạng, và cũng không thể: tiến trình elevated không nhìn thấy ổ mạng
([CHECK-007](./docs/check.md#check-007)).

Muốn tắt phần nào:

```powershell
# tat tu khoi dong
Remove-Item (Join-Path ([Environment]::GetFolderPath('Startup')) 'MediaFinder.lnk')

# tat tu cap nhat
Unregister-ScheduledTask -TaskName 'MediaFinder - cap nhat chi muc' -Confirm:$false
```

## Vòng kiểm tra

Bốn lệnh, chạy trước mỗi lần commit:

```bash
cd src-tauri && cargo test            # 186 test
cd src-tauri && cargo clippy --all-targets
cd src-tauri && cargo fmt --check     # phải im lặng
npm run check                         # type-check frontend
```

Định dạng theo **mặc định của rustfmt**, không có `rustfmt.toml`. Đã đo: không cấu hình nào khớp
với mã nguồn tốt hơn mặc định — xem [`docs/config.md`](./docs/config.md#conf-005).

Hai chỗ mang `#[rustfmt::skip]`, và chỉ hai chỗ: `is_word_boundary` trong `index/search.rs` và
`mod pkey` trong `media/metadata.rs`. Cả hai là **bảng dữ liệu** mà đọc theo hàng mới có nghĩa;
rustfmt tách mỗi phần tử một dòng và làm mất hình dạng của bảng.

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

10. **Gọi cửa sổ xong phải gõ được ngay.** `summon()` không chỉ hiện cửa sổ mà còn phát sự
    kiện `summon` để giao diện đặt con trỏ vào ô tìm kiếm và bôi đen nội dung cũ. Hiện cửa
    sổ mà con trỏ nằm chỗ khác thì phím tắt gần như vô dụng — xem
    [`docs/bug.md`](./docs/bug.md#bug-015).

## Cách dùng

| Thao tác | Phím |
|---|---|
| Gọi cửa sổ từ bất kỳ đâu, hoặc ẩn đi | `Ctrl+Alt+Space` |
| Di chuyển trong kết quả | `↑ ↓ ← → PageUp PageDown` |
| Mở tệp bằng ứng dụng mặc định | `Enter` |
| Mở thư mục chứa tệp trong Explorer | `Ctrl+Enter`, hoặc chuột phải |
| Xoá truy vấn | `Esc` |

Nếu một ứng dụng khác đã chiếm `Ctrl+Alt+Space`, MediaFinder vẫn khởi động bình thường và
nói rõ trên màn hình trống rằng phím tắt không dùng được.

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
