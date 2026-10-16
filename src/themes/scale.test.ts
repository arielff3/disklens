import { describe, expect, it } from "vitest";
import { alpha, mixWhite, parseColor, scale } from "./scale";
import { LAGOON } from "./palettes";

describe("scale", () => {
  it("reproduces the user's palettes from their 500 shade", () => {
    for (const family of Object.values(LAGOON)) {
      expect(scale(family["500"])).toEqual(family);
    }
  });

  it("accepts hex colors", () => {
    expect(parseColor("#f26b2a")).toEqual([242, 107, 42]);
    expect(scale("#808080")["50"]).toBe("rgb(242 242 242)");
  });
});

describe("color helpers", () => {
  it("adds alpha and mixes with white", () => {
    expect(alpha("rgb(0 26 36)", 0.45)).toBe("rgb(0 26 36 / 0.45)");
    expect(mixWhite("rgb(0 0 0)", 0.5)).toBe("rgb(128 128 128)");
  });
});
