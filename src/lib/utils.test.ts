import { describe, it, expect } from "vitest";
import { cn, formatCurrency, formatNumber, formatTokens } from "./utils";

describe("cn (class name merger)", () => {
  it("merges simple classes", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("handles conditional classes", () => {
    expect(cn("base", false && "hidden", "visible")).toBe("base visible");
  });

  it("deduplicates tailwind classes", () => {
    const result = cn("text-red-500", "text-blue-500");
    expect(result).toBe("text-blue-500");
  });
});

describe("formatCurrency", () => {
  it("formats USD amounts", () => {
    expect(formatCurrency(10.5)).toBe("$10.50");
  });

  it("formats zero", () => {
    expect(formatCurrency(0)).toBe("$0.00");
  });

  it("formats large amounts", () => {
    expect(formatCurrency(1234.56)).toContain("1,234.56");
  });
});

describe("formatNumber", () => {
  it("formats small numbers as-is", () => {
    expect(formatNumber(42)).toBe("42");
  });

  it("formats thousands", () => {
    expect(formatNumber(1500)).toBe("1.5K");
  });

  it("formats millions", () => {
    expect(formatNumber(2_500_000)).toBe("2.50M");
  });

  it("formats billions", () => {
    expect(formatNumber(3_000_000_000)).toBe("3.00B");
  });
});

describe("formatTokens", () => {
  it("formats small counts with locale separators", () => {
    expect(formatTokens(1234)).toBe("1,234");
  });

  it("formats wan (10K+)", () => {
    expect(formatTokens(50000)).toContain("5.0");
    expect(formatTokens(50000)).toContain("万");
  });

  it("formats yi (100M+)", () => {
    expect(formatTokens(200_000_000)).toContain("2.00");
    expect(formatTokens(200_000_000)).toContain("亿");
  });

  it("handles zero", () => {
    expect(formatTokens(0)).toBe("0");
  });
});
