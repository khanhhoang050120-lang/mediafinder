// Nối vào recorder mà bài kiểm thử gắn lên globalThis trước khi mount.
export function invoke(cmd: string, args?: unknown): Promise<unknown> {
  const ipc = (globalThis as any).__ipc;
  if (!ipc) return Promise.reject(new Error("recorder chưa gắn (globalThis.__ipc)"));
  return ipc.invoke(cmd, args);
}
export function convertFileSrc(p: string): string {
  return "asset://" + p;
}
