import "@testing-library/jest-dom/vitest";

if (!window.PointerEvent)
  window.PointerEvent = MouseEvent as typeof PointerEvent;

if (!globalThis.ResizeObserver) {
  globalThis.ResizeObserver = class ResizeObserverStub implements ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

if (!window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => undefined,
    removeListener: () => undefined,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    dispatchEvent: () => false,
  })) as typeof window.matchMedia;
}
