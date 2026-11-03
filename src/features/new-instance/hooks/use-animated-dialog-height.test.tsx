import { render } from "@testing-library/react";
import { type RefObject, useRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useAnimatedDialogHeight } from "@/features/new-instance/hooks/use-animated-dialog-height";

let height = 300;
const animate = vi.fn();

function Harness({ view }: { view: string }) {
  const anchor: RefObject<HTMLSpanElement | null> = useRef(null);
  useAnimatedDialogHeight(anchor, view);
  return (
    <div data-slot="dialog-content">
      <span ref={anchor} hidden />
    </div>
  );
}

beforeEach(() => {
  height = 300;
  animate.mockReset();
  // The layout height: a transform such as the open animation's scale must
  // not leak into the measurement.
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(
    () => height,
  );
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    () => ({ height: height * 0.95 }) as DOMRect,
  );
  Object.defineProperty(HTMLElement.prototype, "animate", {
    configurable: true,
    value: animate,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  Reflect.deleteProperty(HTMLElement.prototype, "animate");
  Reflect.deleteProperty(window, "matchMedia");
});

describe("useAnimatedDialogHeight", () => {
  it("does not animate when the popup first opens", () => {
    render(<Harness view="source" />);

    expect(animate).not.toHaveBeenCalled();
  });

  it("grows from the old height to the new one when the view changes", () => {
    const { rerender } = render(<Harness view="source" />);

    height = 640;
    rerender(<Harness view="details" />);

    expect(animate).toHaveBeenCalledOnce();
    expect(animate).toHaveBeenCalledWith(
      [{ height: "300px" }, { height: "640px" }],
      expect.objectContaining({ duration: 200 }),
    );
  });

  it("shrinks again when going back", () => {
    const { rerender } = render(<Harness view="source" />);
    height = 640;
    rerender(<Harness view="details" />);
    animate.mockClear();

    height = 300;
    rerender(<Harness view="source" />);

    expect(animate).toHaveBeenCalledWith(
      [{ height: "640px" }, { height: "300px" }],
      expect.objectContaining({ duration: 200 }),
    );
  });

  it("stays still when the height did not change", () => {
    const { rerender } = render(<Harness view="source" />);

    rerender(<Harness view="details" />);

    expect(animate).not.toHaveBeenCalled();
  });

  it("stays still when the user prefers reduced motion", () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: () => ({ matches: true }),
    });
    const { rerender } = render(<Harness view="source" />);

    height = 640;
    rerender(<Harness view="details" />);

    expect(animate).not.toHaveBeenCalled();
  });

  it("does nothing outside a dialog", () => {
    function Bare({ view }: { view: string }) {
      const anchor: RefObject<HTMLSpanElement | null> = useRef(null);
      useAnimatedDialogHeight(anchor, view);
      return <span ref={anchor} hidden />;
    }
    const { rerender } = render(<Bare view="source" />);

    height = 640;
    rerender(<Bare view="details" />);

    expect(animate).not.toHaveBeenCalled();
  });
});
