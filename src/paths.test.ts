import { describe, expect, it } from "vitest";
import { displayName, isWithin, joinPath, parentPath, samePath } from "./paths";

describe("paths", () => {
  it("joins without doubling the separator", () => {
    expect(joinPath("C:\\", "Users")).toBe("C:\\Users");
    expect(joinPath("C:\\Users", "you")).toBe("C:\\Users\\you");
  });

  it("compares case-insensitively, ignoring a trailing separator", () => {
    expect(samePath("c:\\users\\", "C:\\Users")).toBe(true);
    expect(samePath("C:\\", "c:\\")).toBe(true);
    expect(samePath("C:\\Users", "C:\\Users2")).toBe(false);
  });

  it("checks containment on separator boundaries", () => {
    expect(isWithin("C:\\Users\\a", "C:\\")).toBe(true);
    expect(isWithin("c:\\users", "C:\\Users\\")).toBe(true);
    expect(isWithin("C:\\Users2", "C:\\Users")).toBe(false);
    expect(isWithin("D:\\x", "C:\\")).toBe(false);
  });

  it("shows drives as letters and folders by their last name", () => {
    expect(displayName("C:\\")).toBe("C:");
    expect(displayName("C:\\Users\\you\\Downloads")).toBe("Downloads");
  });

  it("finds the parent folder, stopping at the drive root", () => {
    expect(parentPath("C:\\Users\\you\\a.txt")).toBe("C:\\Users\\you");
    expect(parentPath("C:\\Users\\")).toBe("C:\\");
    expect(parentPath("C:\\Users")).toBe("C:\\");
    expect(parentPath("C:\\")).toBe("C:\\");
  });
});
