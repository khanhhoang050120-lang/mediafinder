//! Does cancelling actually stop the disk?
//!
//! The unit test proves `find_duplicates` returns nothing when the flag is
//! already raised. That is not the question a person cares about: they care
//! whether closing the view stops a scan *already reading*, or leaves it
//! grinding for another ten minutes.
//!
//! ```text
//! cargo test --test dupes_cancel_real -- --ignored --nocapture
//! ```

#![cfg(windows)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use mediafinder::index::persist;
use mediafinder::media::dupes::DupeService;

#[test]
#[ignore = "cần chỉ mục đã quét trên máy thật; chạy với --ignored"]
fn cancelling_stops_a_scan_already_reading() {
    let index = match persist::load() {
        Ok(c) => Arc::new(c.index),
        Err(e) => {
            eprintln!("chưa có cache ({e}) — bỏ qua");
            return;
        }
    };

    let service = DupeService::new();
    assert!(
        service.start(
            Arc::clone(&index),
            0,
            mediafinder::media::dupescope::DupeScope::Everything,
            Vec::new()
        ),
        "quét phải bắt đầu"
    );

    // Let it get properly underway, so this measures stopping real work rather
    // than a scan that had not started reading yet.
    std::thread::sleep(Duration::from_secs(5));
    let mid = service.progress();
    assert!(mid.running, "sau 5 giây quét vẫn phải đang chạy");
    eprintln!("đang chạy: đã đọc {}/{} tệp", mid.hashed, mid.candidates);
    assert!(mid.hashed > 0, "phải đã đọc được tệp nào đó rồi");

    let asked = Instant::now();
    service.cancel();

    // How long until the thread actually notices.
    let mut waited = Duration::ZERO;
    while service.progress().running && waited < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(100));
        waited = asked.elapsed();
    }
    let stopped_after = asked.elapsed();

    let after = service.progress();
    eprintln!("dừng sau {:.2}s", stopped_after.as_secs_f64());
    eprintln!("completed = {} (phải là false)", after.completed);
    eprintln!("groups    = {} (phải là 0)", after.groups);

    assert!(
        !after.running,
        "huỷ rồi thì phải dừng, không được chạy tiếp"
    );
    assert!(
        stopped_after < Duration::from_secs(30),
        "phải dừng trong vòng 30 giây, thực tế {:.1}s",
        stopped_after.as_secs_f64()
    );
    assert!(
        !after.completed,
        "quét bị huỷ không được đánh dấu là đã hoàn tất"
    );
    assert_eq!(
        after.groups, 0,
        "quét bị huỷ không được để lại kết quả dở dang"
    );

    // And a fresh scan must be startable afterwards — a cancel that left the
    // service stuck would be worse than no cancel at all.
    assert!(
        service.start(
            index,
            0,
            mediafinder::media::dupescope::DupeScope::Everything,
            Vec::new()
        ),
        "sau khi huỷ phải bắt đầu lại được lượt quét mới"
    );
    service.cancel();
}
