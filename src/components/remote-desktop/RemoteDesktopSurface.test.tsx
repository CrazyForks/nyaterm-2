import { act, render } from "@testing-library/react";
import { createRef } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RemoteDesktopFramePatch } from "@/lib/remoteDesktopFrame";
import { RemoteDesktopSurface, type RemoteDesktopSurfaceHandle } from "./RemoteDesktopSurface";

const { createRenderer, draw, dispose } = vi.hoisted(() => ({
  createRenderer: vi.fn(),
  draw: vi.fn(),
  dispose: vi.fn(),
}));

vi.mock("./renderer", () => ({
  createRemoteDesktopRenderer: createRenderer,
}));

const patch: RemoteDesktopFramePatch = {
  sequence: 1n,
  desktopWidth: 10,
  desktopHeight: 10,
  x: 0,
  y: 0,
  width: 1,
  height: 1,
  stride: 4,
  pixelFormat: "RGBA8888",
  payload: new Uint8Array([0, 0, 0, 255]),
};

describe("RemoteDesktopSurface", () => {
  beforeEach(() => {
    draw.mockReset();
    dispose.mockReset();
    createRenderer.mockReset();
    createRenderer.mockReturnValue({ draw, dispose });
  });

  it("draws frames imperatively without storing framebuffer data in React state", () => {
    const ref = createRef<RemoteDesktopSurfaceHandle>();
    render(<RemoteDesktopSurface ref={ref} scaleMode="fit" active visible />);

    act(() => ref.current?.drawFrame(patch));
    act(() => ref.current?.drawFrame({ ...patch, sequence: 2n }));

    expect(createRenderer).toHaveBeenCalledOnce();
    expect(draw).toHaveBeenCalledTimes(2);
  });

  it("implements fit, actual, and stretch as local canvas presentation modes", () => {
    const view = render(<RemoteDesktopSurface scaleMode="fit" active visible />);
    const canvas = view.container.querySelector("canvas");
    expect(canvas?.style.objectFit).toBe("contain");
    expect(canvas?.style.width).toBe("100%");

    view.rerender(<RemoteDesktopSurface scaleMode="actual" active visible />);
    expect(canvas?.style.width).toBe("auto");
    expect(canvas?.style.maxWidth).toBe("none");

    view.rerender(<RemoteDesktopSurface scaleMode="stretch" active visible />);
    expect(canvas?.style.width).toBe("100%");
    expect(canvas?.style.height).toBe("100%");
    expect(canvas?.style.objectFit).toBe("");
  });

  it("marks its focus root as a remote desktop input surface", () => {
    const view = render(<RemoteDesktopSurface scaleMode="fit" active visible />);
    expect(view.container.querySelector('[data-remote-desktop-input-root="true"]')).not.toBeNull();
  });

  it("disposes renderer resources on reset and unmount", () => {
    const ref = createRef<RemoteDesktopSurfaceHandle>();
    const view = render(<RemoteDesktopSurface ref={ref} scaleMode="fit" active visible />);
    act(() => ref.current?.drawFrame(patch));
    act(() => ref.current?.reset());
    expect(dispose).toHaveBeenCalledOnce();

    act(() => ref.current?.drawFrame(patch));
    view.unmount();
    expect(dispose).toHaveBeenCalledTimes(2);
  });
});
