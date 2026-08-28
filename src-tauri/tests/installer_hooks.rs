//! Chốt chặn cho `nsis-hooks.nsh` — tệp không có trình biên dịch nào kiểm hộ.
//!
//! Không có makensis trong vòng kiểm tra, nên sai sót ở tệp này chỉ lộ ra lúc
//! CI đóng gói, hoặc tệ hơn: trên máy người dùng. Bài kiểm thử này canh đúng
//! những bất biến mà một lỗi thật đã dạy chúng ta.
//!
//! Lỗi ấy: móc gỡ cài đặt chạy MỖI LẦN uninstaller chạy — kể cả khi chính bộ
//! cài gọi nó để cài đè lên bản cũ. Người dùng tải .exe về cài đè, hộp thoại
//! NSIS mặc định chọn "Uninstall before installing", và móc xoá sạch
//! `index.bin` — gồm cả phần ổ mạng mất vài phút mới quét lại được. Họ chỉ
//! định nâng cấp. Sau đó họ tìm một tệp có thật trên NAS, không thấy, và kết
//! luận bản mới bị hỏng.

const HOOKS: &str = include_str!("../nsis-hooks.nsh");

/// Cắt lấy thân của một macro `!macro TÊN ... !macroend`.
fn macro_body(name: &str) -> &'static str {
    let needle = format!("!macro {name}");
    let start = HOOKS
        .find(&needle)
        .unwrap_or_else(|| panic!("khong tim thay macro {name} trong nsis-hooks.nsh"));
    let rest = &HOOKS[start..];
    let end = rest
        .find("!macroend")
        .unwrap_or_else(|| panic!("macro {name} khong duoc dong bang !macroend"));
    &rest[..end]
}

/// Bất biến quan trọng nhất: **không móc nào được xoá dữ liệu vô điều kiện.**
///
/// Mọi lệnh xoá phải nằm sau một chốt chặn. Kiểm bằng cách đòi hỏi mỗi móc có
/// ít nhất một `${If}` đứng trước lệnh xoá đầu tiên của nó.
#[test]
fn moc_go_cai_dat_khong_xoa_gi_vo_dieu_kien() {
    for name in ["NSIS_HOOK_PREUNINSTALL", "NSIS_HOOK_POSTUNINSTALL"] {
        let body = macro_body(name);
        let first_destructive = ["Delete ", "RMDir", "nsExec::"]
            .iter()
            .filter_map(|k| body.find(k))
            .min();
        let Some(destructive_at) = first_destructive else {
            continue; // móc không làm gì phá huỷ — không cần chốt
        };
        let guard_at = body
            .find("${If}")
            .unwrap_or_else(|| panic!("{name}: co lenh pha huy ma khong co chot chan nao"));
        assert!(
            guard_at < destructive_at,
            "{name}: lenh pha huy dau tien dung TRUOC moi chot chan — \
             cai de len ban cu se xoa mat du lieu nguoi dung"
        );
    }
}

/// Chốt chặn phải nhìn vào cả ba tín hiệu phân biệt "gỡ hẳn" với "gỡ để cài đè".
///
/// Thiếu `$EXEDIR`/`$INSTDIR` là mất tín hiệu quyết định: NSIS chỉ chạy
/// uninstaller **tại chỗ** khi bộ cài gọi kèm `_?=`; người dùng tự gỡ thì nó
/// chạy từ một bản sao trong thư mục tạm.
#[test]
fn chot_chan_nhin_du_ba_tin_hieu() {
    for signal in [
        "$EXEDIR",
        "$INSTDIR",
        "$UpdateMode",
        "$DeleteAppDataCheckboxState",
    ] {
        assert!(
            HOOKS.contains(signal),
            "nsis-hooks.nsh khong con nhin vao {signal} — mat mot tin hieu phan biet \
             go-han voi go-de-cai-de"
        );
    }
}

/// Bản cập nhật trong ứng dụng phải là đường tuyệt đối không đụng dữ liệu.
#[test]
fn che_do_cap_nhat_khong_bao_gio_duoc_xoa() {
    assert!(
        HOOKS.contains("$UpdateMode = 1") || HOOKS.contains("$UpdateMode <> 1"),
        "khong thay dieu kien nao ve $UpdateMode — ban cap nhat co the xoa du lieu"
    );
}

/// Gỡ thật thì phải dọn hết những gì ứng dụng tự tạo ra.
///
/// Danh sách này lớn dần theo thời gian (dấu vết quét mạng, bộ ghi truy vấn,
/// nhật ký). Quên một tệp nghĩa là để lại rác trên hai mươi tới bốn mươi máy
/// mà không ai đi dọn được nữa.
#[test]
fn go_that_thi_don_het_tep_ung_dung_tu_tao() {
    let body = macro_body("NSIS_HOOK_POSTUNINSTALL");
    for f in [
        "index.bin",
        "metadata.bin",
        "progress.json",
        "netscan.json",
        "misses.jsonl",
        "logs",
    ] {
        assert!(
            body.contains(f),
            "POSTUNINSTALL khong don {f} — go cai dat xong van con rac"
        );
    }
}
