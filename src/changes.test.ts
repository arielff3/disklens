import { describe, expect, it } from "vitest";
import { formatDelta, relativeTo } from "./changes";

describe("changes", () => {
  it("formats signed sizes", () => {
    expect(formatDelta(1536 * 1024 ** 2)).toBe("+1.5 GB");
    expect(formatDelta(-200 * 1024 ** 2)).toBe("−200 MB");
    expect(formatDelta(0)).toBe("0 B");
  });

  it("shows paths relative to the scanned root", () => {
    expect(relativeTo("C:\\Users\\a\\Docker", "C:\\")).toBe("Users\\a\\Docker");
    expect(relativeTo("C:\\Users\\a\\Docker", "C:\\Users\\a")).toBe("Docker");
    expect(relativeTo("D:\\x", "C:\\")).toBe("D:\\x");
  });
});
