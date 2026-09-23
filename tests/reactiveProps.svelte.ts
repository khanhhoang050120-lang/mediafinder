/// Props có phản ứng cho bài kiểm thử: đổi một trường là component thấy.
///
/// Tệp `.svelte.ts` vì `$state` chỉ dùng được trong mã Svelte biên dịch — tệp
/// `.test.ts` thường không có nó, nên không đổi được prop sau khi `mount`.
export function reactiveProps<T extends object>(init: T): T {
  const p = $state(init);
  return p;
}
