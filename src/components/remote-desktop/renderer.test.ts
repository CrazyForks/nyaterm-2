import { describe, expect, it, vi } from "vitest";
import { Canvas2dRemoteDesktopRenderer, createRemoteDesktopRenderer } from "./renderer";

function patch(pixelFormat: "RGBA8888" | "BGRA8888" = "RGBA8888") {
  return {
    sequence: 1n,
    desktopWidth: 4,
    desktopHeight: 3,
    x: 1,
    y: 1,
    width: 1,
    height: 1,
    stride: 4,
    pixelFormat,
    payload: new Uint8Array([1, 2, 3, 255]),
  } as const;
}

describe("remote desktop renderer", () => {
  it("falls back to Canvas2D when WebGL2 is unavailable", () => {
    const imageData = { data: new Uint8ClampedArray(4) } as ImageData;
    const context = {
      createImageData: vi.fn(() => imageData),
      putImageData: vi.fn(),
    } as unknown as CanvasRenderingContext2D;
    const canvas = document.createElement("canvas");
    vi.spyOn(canvas, "getContext").mockImplementation((type) => (type === "2d" ? context : null));

    const renderer = createRemoteDesktopRenderer(canvas);
    expect(renderer).toBeInstanceOf(Canvas2dRemoteDesktopRenderer);

    renderer?.draw(patch("BGRA8888"));
    expect(canvas.width).toBe(4);
    expect(canvas.height).toBe(3);
    expect([...imageData.data]).toEqual([3, 2, 1, 255]);
    expect(context.putImageData).toHaveBeenCalledWith(imageData, 1, 1);
  });

  it("returns null when neither rendering context is available", () => {
    const canvas = document.createElement("canvas");
    vi.spyOn(canvas, "getContext").mockReturnValue(null);

    expect(createRemoteDesktopRenderer(canvas)).toBeNull();
  });
});
