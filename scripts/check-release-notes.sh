#!/usr/bin/env bash
# Chốt chặn cho RELEASE_NOTES.md — chạy trong release.yml trước khi build.
#
# Nội dung file này hiện NGUYÊN VĂN trong hộp thoại mời cập nhật trên máy
# người dùng. Hộp thoại chịu được văn bản dài (ô ghi chú tự cuộn), nhưng
# "không vỡ khung" và "đáng đọc" là hai chuyện khác nhau: tóm tắt phải đọc
# xong trong ~15 giây. Chi tiết dài bao nhiêu cũng được — viết vào phần mô tả
# trên trang Releases (dưới vạch ---, do workflow ghép), hộp thoại không hiện.
set -euo pipefail

FILE="${1:-RELEASE_NOTES.md}"
MAX_CHARS=1200

if [ ! -f "$FILE" ]; then
  echo "LOI: khong tim thay $FILE — buoc 0 cua quy trinh phat hanh la viet no." >&2
  exit 1
fi

# Đếm ký tự (không tính khoảng trắng đầu/cuối). wc -m đếm theo ký tự đa byte.
CHARS=$(tr -d '[:space:]' < "$FILE" | wc -m)

if [ "$CHARS" -lt 20 ]; then
  echo "LOI: $FILE gan nhu rong ($CHARS ky tu) — quen viet 'co gi moi' cho ban nay?" >&2
  exit 1
fi

if [ "$CHARS" -gt "$MAX_CHARS" ]; then
  echo "LOI: $FILE dai $CHARS ky tu, tran $MAX_CHARS." >&2
  echo "Day la TOM TAT nguoi dung doc trong hop thoai cap nhat (~15 giay)." >&2
  echo "Cat gon lai; chi tiet ky thuat de xuong trang Releases." >&2
  exit 1
fi

echo "OK: ghi chu $CHARS/$MAX_CHARS ky tu."
