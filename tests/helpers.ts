/// Ghi lại mọi lệnh IPC được gọi, để bài kiểm thử khẳng định về thứ tự và số
/// lần gọi. Backend giả: mỗi lệnh trả về giá trị hoặc hàm mà bài đã cài.
export class IpcRecorder {
  calls: { cmd: string; args: unknown }[] = [];
  handlers = new Map<string, unknown>();
  listeners = new Map<string, ((e: { payload: unknown }) => unknown)[]>();

  on(cmd: string, fn: unknown): this {
    this.handlers.set(cmd, fn);
    return this;
  }

  count(cmd: string): number {
    return this.calls.filter((c) => c.cmd === cmd).length;
  }

  reset(): void {
    this.calls = [];
  }

  async invoke(cmd: string, args: unknown): Promise<unknown> {
    this.calls.push({ cmd, args });
    // `has` chứ không phải kiểm tra giá trị: nhiều lệnh trả về null một cách
    // hợp lệ (request_scan, cancel_scan…), và coi null là "chưa mock" thì bài
    // kiểm thử báo lỗi cho đúng cái nó đang muốn kiểm.
    if (!this.handlers.has(cmd)) throw new Error(`IPC chưa mock: ${cmd}`);
    const h = this.handlers.get(cmd);
    return typeof h === "function" ? await (h as (a: unknown) => unknown)(args) : h;
  }

  /// Phát một sự kiện Tauri tới các listener đã đăng ký.
  async emit(event: string, payload: unknown): Promise<void> {
    for (const l of this.listeners.get(event) ?? []) await l({ payload });
  }

  listen(event: string, cb: (e: { payload: unknown }) => unknown): Promise<() => void> {
    const ls = this.listeners.get(event) ?? [];
    ls.push(cb);
    this.listeners.set(event, ls);
    return Promise.resolve(() => {
      this.listeners.set(
        event,
        (this.listeners.get(event) ?? []).filter((x) => x !== cb),
      );
    });
  }
}

/// Nhường vài vòng microtask + timer để các $effect và promise chạy xong.
export async function settle(ms = 0): Promise<void> {
  for (let i = 0; i < 8; i++) await Promise.resolve();
  if (ms > 0) await new Promise((r) => setTimeout(r, ms));
  for (let i = 0; i < 8; i++) await Promise.resolve();
}

/// Bộ gom kết quả cho các nhóm kịch bản dài: một nhóm là một `it`, nhưng khi
/// vỡ thì tên của TỪNG ca hỏng hiện ra trong thông báo, không chỉ ca đầu tiên.
export function makeCollector() {
  const results: string[] = [];
  let pass = 0;
  const check = (name: string, cond: unknown, detail = "") => {
    if (cond) {
      pass++;
      results.push(`  PASS  ${name}`);
    } else {
      results.push(`  FAIL  ${name}${detail ? "\n          " + detail : ""}`);
    }
  };
  const finish = () => {
    const failed = results.filter((r) => r.includes("FAIL"));
    if (failed.length) {
      throw new Error(`${failed.length} ca hỏng, ${pass} ca đạt:\n` + results.join("\n"));
    }
    return pass;
  };
  return { check, finish, results };
}
