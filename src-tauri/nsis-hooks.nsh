; Móc cho bộ cài NSIS.
;
; Chỉ có một việc: khi gỡ ứng dụng thì gỡ luôn những gì nó đã tự tạo ra ngoài
; thư mục cài đặt. Bộ cài NSIS chỉ biết xoá những tệp chính nó đã chép vào; lối
; tắt tự khởi động và tác vụ đã lên lịch thì do ứng dụng tạo lúc chạy, nên nếu
; không có móc này chúng sẽ ở lại vĩnh viễn — một tác vụ chạy mỗi ngày để khởi
; động một chương trình không còn tồn tại.
;
; Trên hai mươi tới bốn mươi máy thì thứ rác đó không tự dọn được nữa.

!macro NSIS_HOOK_PREUNINSTALL
  ; Chính ứng dụng biết nó đã tạo những gì, nên để nó tự dọn. Chạy trước khi
  ; tệp bị xoá — sau đó thì không còn gì để chạy.
  ;
  ; Không kiểm mã trả về: xoá tác vụ đã lên lịch cần quyền Administrator, mà bộ
  ; gỡ cài đặt ở chế độ currentUser thì không có. Thất bại ở đây chỉ để lại một
  ; tác vụ vô hại sẽ tự lỗi, và không đáng để chặn việc gỡ cài đặt.
  ; Gỡ im lặng thì truyền --quiet: lúc đó không có ai ngồi trước máy để trả lời
  ; hộp thoại xin quyền, và một hộp thoại không ai bấm sẽ treo cả tiến trình gỡ.
  ${If} ${Silent}
    nsExec::ExecToStack '"$INSTDIR\mediafinder.exe" --remove-setup --quiet'
  ${Else}
    nsExec::ExecToStack '"$INSTDIR\mediafinder.exe" --remove-setup'
  ${EndIf}
  Pop $0
  Pop $1
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Chỉ mục nằm cùng thư mục với chương trình (RISK-004) và bộ cài không biết
  ; tới chúng vì không phải nó chép vào. Gỡ cài đặt nghĩa là gỡ hẳn, nên xoá
  ; luôn — để lại 45 MB dữ liệu chết trên mỗi máy là không chấp nhận được.
  Delete "$INSTDIR\index.bin"
  Delete "$INSTDIR\metadata.bin"
  Delete "$INSTDIR\progress.json"
  RMDir "$INSTDIR"
!macroend
