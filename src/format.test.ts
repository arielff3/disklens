import { describe, expect, it } from "vitest";
import { formatAge, formatBytes, formatFiles, splitBytes } from "./format";

const MB = 1024 ** 2;
const GB = 1024 ** 3;

describe("formatBytes", () => {
  it("formats with 1024-based units like Windows Explorer", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(700 * MB)).toBe("700 MB");
    expect(formatBytes(5.35 * GB)).toBe("5.35 GB");
    expect(formatBytes(37.25 * GB)).toBe("37.25 GB");
    expect(formatBytes(2 * 1024 ** 4)).toBe("2 TB");
  });

  it("uses one decimal from 100 up", () => {
    expect(formatBytes(123.45 * GB)).toBe("123.5 GB");
  });

  it("splits value and unit", () => {
    expect(splitBytes(13.4 * GB)).toEqual({ value: "13.4", unit: "GB" });
  });
});

describe("formatFiles", () => {
  it("pluralizes and groups thousands", () => {
    expect(formatFiles(1)).toBe("1 file");
    expect(formatFiles(495219)).toBe("495,219 files");
  });
});

describe("formatAge", () => {
  const now = 1_800_000_000;
  const DAY = 86_400;
  it("says how long ago, in the largest whole unit", () => {
    expect(formatAge(now - 60, now)).toBe("today");
    expect(formatAge(now - DAY, now)).toBe("yesterday");
    expect(formatAge(now - 5 * DAY, now)).toBe("5 days ago");
    expect(formatAge(now - 45 * DAY, now)).toBe("1 month ago");
    expect(formatAge(now - 200 * DAY, now)).toBe("6 months ago");
    expect(formatAge(now - 365 * DAY, now)).toBe("1 year ago");
    expect(formatAge(now - 3 * 365 * DAY, now)).toBe("3 years ago");
  });

  it("handles unknown and future times", () => {
    expect(formatAge(0, now)).toBe("");
    expect(formatAge(now + 100, now)).toBe("today");
  });
});
