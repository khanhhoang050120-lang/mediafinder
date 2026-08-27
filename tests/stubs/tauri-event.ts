export function listen(event: string, cb: (e: { payload: unknown }) => unknown): Promise<() => void> {
  const ipc = (globalThis as any).__ipc;
  if (!ipc) return Promise.resolve(() => {});
  return ipc.listen(event, cb);
}
export function emit(): Promise<void> {
  return Promise.resolve();
}
