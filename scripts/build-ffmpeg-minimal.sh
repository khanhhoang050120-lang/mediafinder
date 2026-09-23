#!/usr/bin/env bash
#
# Dựng ffmpeg.exe + ffprobe.exe tối giản để bộ cài MediaFinder mang theo.
#
# # Vì sao không dùng bản phân phối sẵn
#
# Bản "full" nặng 242 MB — hơn cả bộ cài 200 MB hiện tại, và mỗi lần cập nhật
# là từng ấy byte qua mạng cho 20–40 máy studio. MediaFinder chỉ cần một phần
# rất nhỏ: giải mã những codec có trong thư viện, mã hoá H.264 (xem trước) và
# PNG (ảnh thu nhỏ), đọc vài loại container, ghi MP4 phân mảnh.
#
# # Ba cái bẫy mà bản đầu của script này mắc cả ba
#
# * `--disable-autodetect` tắt luôn **đa luồng** (luồng nằm trong danh sách
#   tự dò của configure). ffmpeg vẫn chạy — chỉ là giải mã ProRes 4K trên đúng
#   một lõi. Bật lại bằng `--enable-w32threads`.
# * Thiếu `-static` thì exe đòi `libwinpthread-1.dll`, `zlib1.dll`… của MSYS2.
#   Trên máy dựng thì chạy tốt (DLL nằm trong PATH của MSYS2), còn trên máy
#   người dùng thì **không mở được**. Bước kiểm ở cuối chặn đúng lỗi này.
# * Bộ mã hoá PNG cần zlib. Thiếu zlib thì configure **lặng lẽ bỏ** PNG, và
#   ảnh thu nhỏ bằng ffmpeg biến mất mà không có lỗi nào.
#
# # Chạy ở đâu
#
# Trong shell MINGW64 của MSYS2 — trên máy, hoặc `msys2/setup-msys2` trong
# release.yml (bước đó dùng đúng danh sách gói dưới đây):
#
#   pacman -S --needed git make diffutils nasm \
#       mingw-w64-x86_64-gcc mingw-w64-x86_64-zlib mingw-w64-x86_64-pkgconf
#   bash scripts/build-ffmpeg-minimal.sh
#
# Kết quả nằm ở src-tauri/binaries/{ffmpeg,ffprobe}.exe, kèm LICENSE-ffmpeg.txt.
# Khoảng 10 phút trên máy 12 luồng. release.yml nhớ kết quả theo mã băm của
# tệp này — sửa bất cứ gì ở đây là CI dựng lại.
#
# Đo trên bản dựng ngày 23/9/2026: mỗi exe ~10 MB (bản full 242 MB), chuyển
# mã ProRes 4K và mảnh đầu tiên nhanh ngang bản full 8.1.2 — xem
# src-tauri/binaries/README.md.
#
# # Giấy phép
#
# x264 là GPL, nên ffmpeg dựng kèm nó cũng là GPL. Chạy như một chương trình
# riêng (MediaFinder gọi nó qua tiến trình con, không liên kết) nên mã của
# MediaFinder vẫn giữ Apache-2.0. Nhưng phát hành bản dựng này thì phải kèm
# giấy phép và chỉ đúng mã nguồn đã dựng — hai phiên bản dưới đây được ghim
# chính vì thế, và LICENSE-ffmpeg.txt ghi lại chúng.

set -euo pipefail

THU_MUC_GOC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DICH="$THU_MUC_GOC/src-tauri/binaries"
LAM_VIEC="${LAM_VIEC:-${TMPDIR:-/tmp}/mediafinder-ffmpeg}"

# Ghim chính xác. Một bản dựng phải lặp lại được, và phải chỉ ra được đúng mã
# nguồn đã dựng — "mới nhất" thì hôm nay khác hôm qua.
FFMPEG_TAG="n7.1.1"
X264_COMMIT="b35605ace3ddf7c1a5d67a2eb553f034aef41d55" # đầu nhánh stable, tra ngày 23/9/2026

mkdir -p "$LAM_VIEC" "$DICH"
cd "$LAM_VIEC"

if [ ! -d ffmpeg ]; then
  echo "==> tải ffmpeg $FFMPEG_TAG"
  # Bản sao chính thức trên GitHub, không phải git.ffmpeg.org: release.yml chạy
  # trên máy của GitHub, và tải từ chính GitHub nhanh và ổn định hơn. Cùng một
  # commit — tag n7.1.1 ở cả hai nơi trỏ tới db69d06e (tra ngày 23/9/2026).
  git clone --depth 1 --branch "$FFMPEG_TAG" https://github.com/FFmpeg/FFmpeg.git ffmpeg
fi
if [ ! -d x264 ]; then
  echo "==> tải x264 $X264_COMMIT"
  git init -q x264
  git -C x264 fetch -q --depth 1 https://code.videolan.org/videolan/x264.git "$X264_COMMIT"
  git -C x264 checkout -q FETCH_HEAD
fi

TIEN_TO="$LAM_VIEC/dungxong"
mkdir -p "$TIEN_TO"
SO_LUONG="$(nproc)"

echo "==> dựng x264 (tĩnh, chỉ 8-bit — MediaFinder luôn mã hoá yuv420p)"
cd "$LAM_VIEC/x264"
./configure --prefix="$TIEN_TO" --enable-static --disable-cli --disable-opencl \
  --bit-depth=8 --chroma-format=420
make -j"$SO_LUONG"
make install

echo "==> cấu hình ffmpeg"
cd "$LAM_VIEC/ffmpeg"
export PKG_CONFIG_PATH="$TIEN_TO/lib/pkgconfig"

# Tắt HẾT rồi bật lại đúng thứ cần. Danh sách cấm thì mỗi bản ffmpeg mới lại
# mang thêm thứ không ai xin; danh sách cho phép thì không.
#
# Các codec được chọn theo hai nguồn: thứ đo được trong thư viện thật (ProRes
# chiếm 2/3 số .mov, còn lại chủ yếu H.264), và thứ hay gặp trong .mov/.mkv/
# .avi của một studio dựng phim (DNxHD, HEVC, PCM big-endian của QuickTime…).
./configure \
  --prefix="$TIEN_TO" \
  --pkg-config-flags="--static" \
  --extra-cflags="-I$TIEN_TO/include" \
  --extra-ldflags="-L$TIEN_TO/lib -static" \
  --disable-everything \
  --disable-autodetect \
  --enable-w32threads \
  --enable-zlib \
  --disable-doc \
  --disable-network \
  --disable-debug \
  --disable-ffplay \
  --enable-gpl \
  --enable-libx264 \
  --enable-decoder=prores,h264,hevc,dnxhd,mpeg1video,mpeg2video,mpeg4,msmpeg4v3,h263,vp8,vp9 \
  --enable-decoder=mjpeg,png,qtrle,rawvideo,v210,cfhd,dvvideo,wmv3,vc1,flv \
  --enable-decoder=aac,mp3float,mp2float,ac3,eac3,alac,flac,opus,vorbis,wmav2 \
  --enable-decoder=pcm_s16le,pcm_s16be,pcm_s24le,pcm_s24be,pcm_s32le,pcm_s32be,pcm_f32le,pcm_f32be,pcm_u8 \
  --enable-encoder=libx264,aac,png \
  --enable-demuxer=mov,matroska,avi,mpegts,mpegps,asf,flv \
  --enable-muxer=mp4,image2,image2pipe \
  --enable-parser=h264,hevc,aac,ac3,mpegaudio,mpegvideo,mpeg4video,vp8,vp9,dnxhd,png,mjpeg,vc1,flac,opus,vorbis \
  --enable-filter=scale,format,null,anull,aformat,aresample,buffer,buffersink,abuffer,abuffersink \
  --enable-protocol=file,pipe

echo "==> biên dịch"
make -j"$SO_LUONG"

cp -f ffmpeg.exe ffprobe.exe "$DICH/"
strip "$DICH/ffmpeg.exe" "$DICH/ffprobe.exe"

echo "==> đóng gói mã nguồn đúng bản vừa dựng"
# GPL đòi người nhận bản nhị phân lấy được mã nguồn tương ứng — và "tương
# ứng" gồm cả script điều khiển việc dựng. Cách chắc nhất là cho tải **từ
# cùng một chỗ** với bộ cài: release.yml đính kèm ba tệp này vào đúng trang
# Release chứa bộ cài. `git archive` lấy đúng cây mã ở tag/commit đã dựng,
# không lẫn sản phẩm dựng nào.
NGUON="$DICH/nguon"
rm -rf "$NGUON"
mkdir -p "$NGUON"
X264_NGAN="${X264_COMMIT:0:12}"
git -C "$LAM_VIEC/ffmpeg" archive --format=tar.gz --prefix="ffmpeg-$FFMPEG_TAG/" \
  -o "$NGUON/ffmpeg-$FFMPEG_TAG-source.tar.gz" HEAD
git -C "$LAM_VIEC/x264" archive --format=tar.gz --prefix="x264-$X264_NGAN/" \
  -o "$NGUON/x264-$X264_NGAN-source.tar.gz" HEAD
cp -f "$THU_MUC_GOC/scripts/build-ffmpeg-minimal.sh" "$NGUON/"

# Giấy phép đi kèm bộ cài: lời dẫn + TOÀN VĂN GPL-2.0. GPL đòi đưa người nhận
# một bản giấy phép cùng chương trình — một đường link không đủ. Lấy từ chính
# mã nguồn ffmpeg vừa dựng, nên luôn khớp phiên bản.
{
  cat << EOF
ffmpeg.exe và ffprobe.exe đi kèm MediaFinder

Dựng từ mã nguồn:
  FFmpeg  $FFMPEG_TAG                https://github.com/FFmpeg/FFmpeg
  x264    $X264_COMMIT  https://code.videolan.org/videolan/x264.git

Mã nguồn đúng bản này nằm ngay trên trang GitHub Release chứa bộ cài:
  ffmpeg-$FFMPEG_TAG-source.tar.gz
  x264-$X264_NGAN-source.tar.gz
  build-ffmpeg-minimal.sh   (cách dựng — cũng có trong mã nguồn MediaFinder)

Bản dựng này có libx264 nên phát hành theo GNU General Public License
phiên bản 2 trở lên (GPL-2.0-or-later); toàn văn ở cuối tệp này.

MediaFinder gọi ffmpeg như một chương trình riêng; mã của MediaFinder vẫn
theo Apache License 2.0.

================================================================================

EOF
  cat "$LAM_VIEC/ffmpeg/COPYING.GPLv2"
} > "$DICH/LICENSE-ffmpeg.txt"

echo "==> kiểm: chỉ được phụ thuộc DLL có sẵn trong Windows"
# Một exe đòi DLL của MSYS2 chạy tốt trên máy dựng và hỏng trên mọi máy khác
# — đúng loại lỗi không test nào trên máy dựng bắt được.
LA=0
for exe in "$DICH/ffmpeg.exe" "$DICH/ffprobe.exe"; do
  for dll in $(objdump -p "$exe" | awk '/DLL Name/ {print $3}'); do
    case "$(echo "$dll" | tr '[:upper:]' '[:lower:]')" in
      kernel32.dll | user32.dll | advapi32.dll | shell32.dll | ole32.dll | \
      bcrypt.dll | ws2_32.dll | msvcrt.dll | ucrtbase.dll | api-ms-win-*) ;;
      *) echo "   LỖI: $(basename "$exe") cần $dll"; LA=1 ;;
    esac
  done
done
[ "$LA" = 0 ] || { echo "==> THẤT BẠI: exe không chạy được trên máy không có MSYS2"; exit 1; }

echo "==> kiểm: đủ những thứ MediaFinder gọi tới"
for thu in "decoders prores" "decoders h264" "encoders libx264" "encoders png" "encoders aac" \
           "muxers mp4" "muxers image2" "demuxers mov" "filters scale"; do
  read -r loai ten <<< "$thu"
  "$DICH/ffmpeg.exe" -hide_banner "-$loai" 2>/dev/null | grep -qw "$ten" \
    || { echo "   LỖI: thiếu $loai $ten"; exit 1; }
done
"$DICH/ffmpeg.exe" -hide_banner -buildconf 2>/dev/null | grep -q "enable-w32threads" \
  || { echo "   LỖI: không có đa luồng — giải mã ProRes sẽ chạy trên một lõi"; exit 1; }

echo "==> kiểm: mã nguồn và giấy phép đi kèm"
for t in "$NGUON/ffmpeg-$FFMPEG_TAG-source.tar.gz" "$NGUON/x264-$X264_NGAN-source.tar.gz"; do
  # `tar -tzf` đọc trọn tệp nén: một tệp cụt hay hỏng thì hỏng ngay ở đây.
  tar -tzf "$t" > /dev/null 2>&1 || { echo "   LỖI: mã nguồn hỏng hoặc thiếu: $t"; exit 1; }
done
grep -q "GNU GENERAL PUBLIC LICENSE" "$DICH/LICENSE-ffmpeg.txt" \
  || { echo "   LỖI: LICENSE-ffmpeg.txt thiếu toàn văn GPL"; exit 1; }

echo
echo "==> xong"
ls -l "$DICH"/ffmpeg.exe "$DICH"/ffprobe.exe "$DICH"/LICENSE-ffmpeg.txt "$NGUON"/*
