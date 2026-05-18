import { describe, expect, it } from "vitest";
import { indexToLetter } from "../../src/components/Grid";

describe("indexToLetter", () => {
  it("returns A for 0", () => {
    expect(indexToLetter(0)).toBe("A");
  });

  it("returns Z for 25", () => {
    expect(indexToLetter(25)).toBe("Z");
  });

  it("returns AA for 26", () => {
    expect(indexToLetter(26)).toBe("AA");
  });

  it("returns AB for 27", () => {
    expect(indexToLetter(27)).toBe("AB");
  });

  it("returns AZ for 51", () => {
    expect(indexToLetter(51)).toBe("AZ");
  });

  it("returns BA for 52", () => {
    expect(indexToLetter(52)).toBe("BA");
  });

  it("returns ZZ for 701", () => {
    expect(indexToLetter(701)).toBe("ZZ");
  });

  it("returns AAA for 702", () => {
    expect(indexToLetter(702)).toBe("AAA");
  });

  it("clamps negative inputs to A", () => {
    expect(indexToLetter(-1)).toBe("A");
    expect(indexToLetter(-100)).toBe("A");
  });

  it("floors fractional indices", () => {
    expect(indexToLetter(1.7)).toBe("B");
    expect(indexToLetter(25.9)).toBe("Z");
  });
});
