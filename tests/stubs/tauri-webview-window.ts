export const getCurrentWebviewWindow = () => ({
  listen: () => Promise.resolve(() => {}),
  onDragDropEvent: () => Promise.resolve(() => {}),
});
export class WebviewWindow {}
