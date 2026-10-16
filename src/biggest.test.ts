import { describe, expect, it } from "vitest";
import { NO_CHIPS, toFilter } from "./biggest";

const MB = 1024 ** 2;
const GB = 1024 ** 3;

describe("Biggest chips", () => {
  it("map to the backend filter", () => {
    expect(toFilter(NO_CHIPS)).toEqual({ minSize: null, olderThanDays: null, category: null });
    expect(toFilter({ size: "100mb", old: false, category: null }).minSize).toBe(100 * MB);
    expect(toFilter({ size: "1gb", old: true, category: "video" })).toEqual({
      minSize: GB,
      olderThanDays: 365,
      category: "video",
    });
  });
});
