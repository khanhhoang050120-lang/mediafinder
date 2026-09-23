# Nhị phân ffmpeg đi kèm bộ cài

Thư mục này chứa `ffmpeg.exe`, `ffprobe.exe` và `LICENSE-ffmpeg.txt` mà bộ cài
mang theo, cùng thư mục `nguon/` (mã nguồn để đính kèm Release — xem "Giấy
phép"). Tất cả bị `.gitignore` loại: chúng được **dựng** từ mã nguồn, không
được commit.

## Vì sao cần

Windows không có bộ giải mã Apple ProRes. Đo trên thư viện studio: **2/3 số
tệp `.mov`** là ProRes, và với chúng Windows không dựng được ảnh thu nhỏ
(`WTS_E_FAILEDEXTRACTION`) còn WebView2 chiếu một hình chữ nhật đen mà không
báo lỗi gì. ffmpeg đọc được, nên app dùng nó — xem `src/media/ffmpeg.rs`.

## Dựng

```sh
bash scripts/build-ffmpeg-minimal.sh
```

Chạy trong shell MINGW64 của MSYS2; danh sách gói cần cài nằm ở đầu script.
Khoảng 10 phút trên máy 12 luồng. Kết quả đo được:

| | Bản full (winget) | Bản tối giản |
| --- | --- | --- |
| `ffmpeg.exe` | 242 MB (cả thư mục) | 10,3 MB |
| `ffprobe.exe` | | 10,2 MB |
| Chuyển mã ProRes 4K 18 s (đĩa ấm) | 5,4–5,7 s | 5,5–5,6 s |
| Mảnh đầu tiên | 0,31 s | 0,34 s |
| DLL phụ thuộc | | chỉ DLL có sẵn trong Windows |

## Đóng gói

Chỉ bản phát hành mang ffmpeg. `release.yml` dựng nó (hoặc lấy từ cache theo
mã băm của script) rồi build với cấu hình bổ sung:

```sh
npx tauri build --config src-tauri/tauri.ffmpeg.conf.json
```

`tauri.ffmpeg.conf.json` ánh xạ ba tệp này vào **ngay cạnh** `mediafinder.exe`
trong thư mục cài — đúng chỗ `ffmpeg.rs` tìm đầu tiên.

### Vì sao không khai thẳng trong `tauri.conf.json`

Khai tệp ở `bundle.resources` thì `tauri-build` **đòi tệp phải có mặt lúc
biên dịch** — kể cả `cargo test`. Trên máy CI của `check.yml` hay một bản
clone mới, không ai dựng ffmpeg, và dự án sẽ không biên dịch được:

```text
tauri-build failed: resource path `binaries\ffmpeg.exe` doesn't exist
```

Tách ra một tệp cấu hình riêng thì chỉ lệnh build phát hành đòi tệp.

## Khi chạy từ mã nguồn

`cargo run` / `npm run tauri dev` không mang ffmpeg. App tự dò theo thứ tự:
cạnh tệp exe → `PATH`. Muốn thử đúng bản tối giản thì chạy
`npx tauri dev --config src-tauri/tauri.ffmpeg.conf.json` — Tauri chép ba tệp
vào cạnh exe trong `target/debug`. Không có ffmpeg ở đâu cả thì app quay về
hành vi cũ: huy hiệu màu thay ảnh thu nhỏ, và khung xem trước báo không xem
trước được.

## Giấy phép

Bản dựng có libx264 nên là GPL-2.0-or-later. MediaFinder gọi ffmpeg như một
chương trình riêng nên mã của MediaFinder vẫn giữ Apache-2.0.

GPL đòi hai thứ đi cùng bản nhị phân, và cả hai đều do script tạo ra:

* **Toàn văn giấy phép** — `LICENSE-ffmpeg.txt` (đi kèm bộ cài, nằm cạnh
  `ffmpeg.exe`) gồm lời dẫn + toàn văn GPL-2.0 lấy từ chính mã nguồn ffmpeg.
* **Mã nguồn tương ứng** — `nguon/` chứa mã nguồn ffmpeg và x264 đúng
  tag/commit đã dựng (`git archive`) cùng script dựng. `release.yml` đính kèm
  ba tệp này vào **đúng trang Release chứa bộ cài**, tức người tải bộ cài lấy
  được mã nguồn từ cùng một chỗ.

`release.yml` từ chối phát hành nếu thiếu một trong hai.
