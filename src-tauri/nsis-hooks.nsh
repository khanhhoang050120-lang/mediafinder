; Móc cho bộ cài NSIS.
;
; Việc chính: khi gỡ ứng dụng thì gỡ luôn những gì nó đã tự tạo ra ngoài thư
; mục cài đặt. Bộ cài NSIS chỉ biết xoá những tệp chính nó đã chép vào; lối tắt
; tự khởi động và tác vụ đã lên lịch thì do ứng dụng tạo lúc chạy, nên nếu
; không có móc này chúng sẽ ở lại vĩnh viễn — một tác vụ chạy mỗi ngày để khởi
; động một chương trình không còn tồn tại.
;
; Trên hai mươi tới bốn mươi máy thì thứ rác đó không tự dọn được nữa.
;
; ---------------------------------------------------------------------------
; NHƯNG: hai móc này chạy MỖI LẦN uninstaller chạy — kể cả khi chính bộ cài gọi
; nó để cài đè lên bản cũ.
;
; Bản đầu không phân biệt hai trường hợp đó, và cái giá là một lỗi thật trên
; máy người dùng: tải tệp .exe về cài đè lên bản cũ thì hộp thoại của NSIS mặc
; định chọn "Uninstall before installing", uninstaller chạy, và móc này xoá
; sạch index.bin — bao gồm cả phần ổ mạng phải mất vài phút mới quét lại được.
; Người dùng chỉ định nâng cấp, không định mất dữ liệu. Sau đó họ tìm một tệp
; có thật trên NAS và không thấy đâu cả, rồi kết luận bản mới bị hỏng.
;
; Ba tín hiệu phân biệt "gỡ hẳn" với "gỡ để cài đè":
;
; * `$UpdateMode` — bộ cập nhật trong ứng dụng truyền `/UPDATE`. (Đường này
;   thực ra không chạy uninstaller, nhưng chốt lại cho chắc.)
; * `$EXEDIR` so với `$INSTDIR` — đây là tín hiệu quyết định. NSIS chỉ chạy
;   uninstaller **tại chỗ** khi được gọi kèm `_?=`, và chỉ bộ cài mới gọi kiểu
;   đó. Người dùng tự gỡ thì NSIS chép uninstaller sang thư mục tạm rồi chạy
;   bản sao, nên `$EXEDIR` là thư mục tạm chứ không phải thư mục cài đặt.
; * `$DeleteAppDataCheckboxState` — ô "xoá dữ liệu ứng dụng" người dùng tự
;   tích. Tích rồi thì tôn trọng ý đó, dù đang ở đường nào.
; ---------------------------------------------------------------------------

; Đặt `$R9` = 1 khi lượt gỡ này là gỡ thật sự, được phép dọn dữ liệu.
;
; Tính lại ở cả hai móc thay vì nhớ qua một biến dùng chung: hai móc chạy cách
; nhau cả một Section, và một biến sống qua quãng đó là thứ dễ bị đè nhất.
!macro MF_DECIDE_REAL_UNINSTALL
  StrCpy $R9 0

  ; Người dùng tự gỡ — uninstaller đang chạy từ bản sao trong thư mục tạm.
  ${If} $EXEDIR != $INSTDIR
    StrCpy $R9 1
  ${EndIf}

  ; Hoặc họ tự tay yêu cầu xoá dữ liệu.
  ${If} $DeleteAppDataCheckboxState = 1
    StrCpy $R9 1
  ${EndIf}

  ; Nhưng bản cập nhật trong ứng dụng thì tuyệt đối không đụng vào.
  ${If} $UpdateMode = 1
    StrCpy $R9 0
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro MF_DECIDE_REAL_UNINSTALL

  ${If} $R9 = 1
    ; Chính ứng dụng biết nó đã tạo những gì, nên để nó tự dọn. Chạy trước khi
    ; tệp bị xoá — sau đó thì không còn gì để chạy.
    ;
    ; Không kiểm mã trả về: xoá tác vụ đã lên lịch cần quyền Administrator, mà
    ; bộ gỡ cài đặt ở chế độ currentUser thì không có. Thất bại ở đây chỉ để
    ; lại một tác vụ vô hại sẽ tự lỗi, và không đáng để chặn việc gỡ cài đặt.
    ; Gỡ im lặng thì truyền --quiet: lúc đó không có ai ngồi trước máy để trả
    ; lời hộp thoại xin quyền, và một hộp thoại không ai bấm sẽ treo cả tiến
    ; trình gỡ.
    ${If} ${Silent}
      nsExec::ExecToStack '"$INSTDIR\mediafinder.exe" --remove-setup --quiet'
    ${Else}
      nsExec::ExecToStack '"$INSTDIR\mediafinder.exe" --remove-setup'
    ${EndIf}
    Pop $0
    Pop $1
  ${EndIf}
  ; Cài đè thì giữ nguyên tác vụ và lối tắt: tác vụ trỏ vào đúng đường dẫn mà
  ; bản mới sắp ghi đè lên, nên nó vẫn đúng. Gỡ đi ở đây nghĩa là sau khi nâng
  ; cấp, chỉ mục thôi tự làm mới cho tới khi người dùng tự quét lại một lần có
  ; hỏi quyền — một sự cố im lặng không ai báo cho họ.
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro MF_DECIDE_REAL_UNINSTALL

  ${If} $R9 = 1
    ; Chỉ mục nằm cùng thư mục với chương trình (RISK-004) và bộ cài không biết
    ; tới chúng vì không phải nó chép vào. Gỡ cài đặt nghĩa là gỡ hẳn, nên xoá
    ; luôn — để lại 45 MB dữ liệu chết trên mỗi máy là không chấp nhận được.
    Delete "$INSTDIR\index.bin"
    Delete "$INSTDIR\metadata.bin"
    Delete "$INSTDIR\progress.json"
    ; Những tệp nhỏ ứng dụng tự tạo về sau: dấu vết lần quét ổ mạng, bộ ghi
    ; truy vấn không ra kết quả, và nhật ký chẩn đoán.
    Delete "$INSTDIR\netscan.json"
    Delete "$INSTDIR\misses.jsonl"
    Delete "$INSTDIR\misses.enabled"
    RMDir /r "$INSTDIR\logs"
    RMDir "$INSTDIR"
  ${EndIf}
!macroend
