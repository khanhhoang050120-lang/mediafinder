/// Props phản ứng cho kiểm thử component.
///
/// `mount()` của Svelte 5 trả về phần *exports* của component, không phải một
/// proxy của props — nên gán thẳng vào giá trị trả về không làm gì cả. Muốn
/// đổi prop trên một component đang gắn (đúng như khi người dùng bấm mũi tên
/// lướt sang tệp khác mà overlay vẫn nguyên tại chỗ) thì phải truyền vào một
/// đối tượng `$state`.
///
/// Rune chỉ dùng được trong tệp `.svelte.ts`, nên helper nằm riêng ở đây thay
/// vì trong `helpers.ts`. Cùng lối với `src/lib/scanState.svelte.ts`.
///
/// `$state(...)` phải là **khởi tạo của một khai báo biến** — không trả thẳng
/// được, nên phải qua một biến trung gian. Trình biên dịch nói rõ điều đó, và
/// đây là chỗ duy nhất trong dự án cần lách nó.
export function propsPhanUng<T extends object>(init: T): T {
  const s = $state(init);
  return s;
}
