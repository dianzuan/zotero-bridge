import { expect } from "chai";
import { getTextFromChars, findQuoteInChars, getRangeRects } from "../../src/utils/pdf-locate";

/** Helper: build a char object with sensible defaults. */
function ch(
  c: string,
  rect: [number, number, number, number],
  opts?: { spaceAfter?: boolean; lineBreakAfter?: boolean; ignorable?: boolean; rotation?: number; inlineRect?: [number, number, number, number] },
) {
  return { c, rect, ...opts };
}

describe("getTextFromChars", () => {
  it("concatenates character values into a string", () => {
    const chars = [
      ch("H", [0, 0, 5, 10]),
      ch("i", [5, 0, 8, 10]),
    ];
    expect(getTextFromChars(chars)).to.equal("Hi");
  });

  it("inserts space after chars with spaceAfter", () => {
    const chars = [
      ch("H", [0, 0, 5, 10]),
      ch("i", [5, 0, 8, 10], { spaceAfter: true }),
      ch("y", [12, 0, 17, 10]),
      ch("o", [17, 0, 22, 10]),
    ];
    expect(getTextFromChars(chars)).to.equal("Hi yo");
  });

  it("inserts space after chars with lineBreakAfter", () => {
    const chars = [
      ch("A", [0, 0, 5, 10]),
      ch("B", [5, 0, 10, 10], { lineBreakAfter: true }),
      ch("C", [0, 12, 5, 22]),
      ch("D", [5, 12, 10, 22]),
    ];
    expect(getTextFromChars(chars)).to.equal("AB CD");
  });

  it("skips ignorable chars", () => {
    const chars = [
      ch("A", [0, 0, 5, 10]),
      ch("​", [5, 0, 5, 10], { ignorable: true }),
      ch("B", [5, 0, 10, 10]),
    ];
    expect(getTextFromChars(chars)).to.equal("AB");
  });

  it("returns empty string for empty chars array", () => {
    expect(getTextFromChars([])).to.equal("");
  });

  it("handles all-ignorable chars", () => {
    const chars = [
      ch("​", [0, 0, 0, 0], { ignorable: true }),
      ch("﻿", [0, 0, 0, 0], { ignorable: true }),
    ];
    expect(getTextFromChars(chars)).to.equal("");
  });
});

describe("findQuoteInChars", () => {
  it("finds an exact match and returns offsets", () => {
    const chars = [
      ch("H", [0, 0, 5, 10]),
      ch("e", [5, 0, 10, 10]),
      ch("l", [10, 0, 15, 10]),
      ch("l", [15, 0, 20, 10]),
      ch("o", [20, 0, 25, 10]),
    ];
    const result = findQuoteInChars(chars, "ello");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(1);
    expect(result!.offsetEnd).to.equal(4);
  });

  it("matches whitespace-insensitively (collapsed spaces)", () => {
    const chars = [
      ch("a", [0, 0, 5, 10], { spaceAfter: true }),
      ch("b", [10, 0, 15, 10]),
    ];
    // Text is "a b"; quote with extra spaces should still match
    const result = findQuoteInChars(chars, "a   b");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(1);
  });

  it("finds a match spanning a lineBreakAfter boundary", () => {
    // "AB CD" — quote "B C" spans across the line break
    const chars = [
      ch("A", [0, 90, 5, 100]),
      ch("B", [5, 90, 10, 100], { lineBreakAfter: true }),
      ch("C", [0, 78, 5, 88]),
      ch("D", [5, 78, 10, 88]),
    ];
    const result = findQuoteInChars(chars, "B C");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(1);
    expect(result!.offsetEnd).to.equal(2);
  });

  it("matches CJK quote across a line break (no spaces in quote)", () => {
    // CJK text where line break inserts a space: "显 著" in chars, "显著" in quote
    const chars = [
      ch("显", [300, 500, 310, 510], { lineBreakAfter: true }),
      ch("著", [100, 488, 110, 498]),
      ch("正", [110, 488, 120, 498]),
    ];
    // chars text = "显 著正", quote = "显著正" (no space) — should still match
    const result = findQuoteInChars(chars, "显著正");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(2);
  });

  it("preserves English word boundaries in CJK-aware mode", () => {
    // "data analysis" should match "data analysis" but not "dataanalysis" as a substring
    const chars = "data analysis method".split("").map((c, i) => {
      const x = i * 5;
      return ch(c, [x, 0, x + 5, 10], c === " " ? { spaceAfter: false } : undefined);
    });
    // Fix: spaces should come from spaceAfter on preceding char, not be char entries
    const spacedChars = [
      ...("data".split("").map((c, i) => ch(c, [i * 5, 0, i * 5 + 5, 10]))),
      ch("a", [15, 0, 20, 10], { spaceAfter: true }), // replace last 'a' of "data"
    ];
    // Simpler approach: build chars properly
    const properChars = [
      ch("d", [0, 0, 5, 10]),
      ch("a", [5, 0, 10, 10]),
      ch("t", [10, 0, 15, 10]),
      ch("a", [15, 0, 20, 10], { spaceAfter: true }),
      ch("a", [25, 0, 30, 10]),
      ch("n", [30, 0, 35, 10]),
      ch("a", [35, 0, 40, 10]),
      ch("l", [40, 0, 45, 10]),
      ch("y", [45, 0, 50, 10]),
      ch("s", [50, 0, 55, 10]),
      ch("i", [55, 0, 60, 10]),
      ch("s", [60, 0, 65, 10]),
    ];
    // "data analysis" with word boundary — should match
    const result = findQuoteInChars(properChars, "data analysis");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(11);
  });

  it("matches CJK with scattered spaces (PDF artifact)", () => {
    // PDF renders "创新困境" as "创 新 困 境" with spaces between each char
    const chars = [
      ch("创", [0, 0, 10, 10], { spaceAfter: true }),
      ch("新", [15, 0, 25, 10], { spaceAfter: true }),
      ch("困", [30, 0, 40, 10], { spaceAfter: true }),
      ch("境", [45, 0, 55, 10]),
    ];
    const result = findQuoteInChars(chars, "创新困境");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(3);
  });

  it("returns null when quote is not found", () => {
    const chars = [
      ch("A", [0, 0, 5, 10]),
      ch("B", [5, 0, 10, 10]),
    ];
    expect(findQuoteInChars(chars, "XYZ")).to.be.null;
  });

  it("returns null for empty quote", () => {
    const chars = [ch("A", [0, 0, 5, 10])];
    expect(findQuoteInChars(chars, "")).to.be.null;
    expect(findQuoteInChars(chars, "   ")).to.be.null;
  });

  it("returns null for empty chars array", () => {
    expect(findQuoteInChars([], "hello")).to.be.null;
  });

  it("matches when PDF has curly quotes but query has straight quotes", () => {
    // PDF chars use “/”, query uses ASCII "
    const chars = [
      ch("“", [0, 0, 5, 10]),   // "
      ch("创", [5, 0, 15, 10]),
      ch("新", [15, 0, 25, 10]),
      ch("”", [25, 0, 30, 10]), // "
    ];
    const result = findQuoteInChars(chars, '"创新"');
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(3);
  });

  it("matches when query has curly quotes but PDF has straight quotes", () => {
    const chars = [
      ch('"', [0, 0, 5, 10]),
      ch("测", [5, 0, 15, 10]),
      ch("试", [15, 0, 25, 10]),
      ch('"', [25, 0, 30, 10]),
    ];
    const result = findQuoteInChars(chars, "“测试”");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(3);
  });

  it("matches fullwidth quotation marks against curly quotes", () => {
    const chars = [
      ch("＂", [0, 0, 5, 10]),   // fullwidth "
      ch("数", [5, 0, 15, 10]),
      ch("据", [15, 0, 25, 10]),
      ch("＂", [25, 0, 30, 10]),
    ];
    const result = findQuoteInChars(chars, "“数据”");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(3);
  });

  it("normalizes single quote variants", () => {
    const chars = [
      ch("‘", [0, 0, 5, 10]),   // '
      ch("x", [5, 0, 10, 10]),
      ch("’", [10, 0, 15, 10]), // '
    ];
    const result = findQuoteInChars(chars, "'x'");
    expect(result).to.not.be.null;
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(2);
  });

  it("skips ignorable chars when building text and adjusts offsets", () => {
    const chars = [
      ch("H", [0, 0, 5, 10]),
      ch("​", [5, 0, 5, 10], { ignorable: true }),
      ch("i", [5, 0, 10, 10]),
    ];
    // Text is "Hi" — ignorable char at index 1 is skipped
    const result = findQuoteInChars(chars, "Hi");
    expect(result).to.not.be.null;
    // offsetStart should point to char index 0 ("H"), offsetEnd to char index 2 ("i")
    expect(result!.offsetStart).to.equal(0);
    expect(result!.offsetEnd).to.equal(2);
  });
});

describe("getRangeRects", () => {
  it("returns a single rect for a single-line range", () => {
    const chars = [
      ch("A", [0, 90, 5, 100], { inlineRect: [0, 88, 200, 102] }),
      ch("B", [5, 90, 10, 100], { inlineRect: [0, 88, 200, 102] }),
      ch("C", [10, 90, 15, 100], { inlineRect: [0, 88, 200, 102], lineBreakAfter: true }),
    ];
    // Range covers chars 0..2 (entire line)
    const rects = getRangeRects(chars, 0, 2);
    expect(rects).to.have.lengthOf(1);
    const r = rects[0];
    // x from firstRect[0]=0, y from firstInline[1]=88, x2 from lastRect[2]=15, y2 from firstInline[3]=102
    expect(r).to.deep.equal([0, 88, 15, 102]);
  });

  it("returns multiple rects for a multi-line range", () => {
    const chars = [
      ch("A", [0, 90, 5, 100], { inlineRect: [0, 88, 200, 102], lineBreakAfter: true }),
      ch("B", [0, 70, 5, 80], { inlineRect: [0, 68, 200, 82], lineBreakAfter: true }),
    ];
    const rects = getRangeRects(chars, 0, 1);
    expect(rects).to.have.lengthOf(2);
    // First line rect
    expect(rects[0]).to.deep.equal([0, 88, 5, 102]);
    // Second line rect
    expect(rects[1]).to.deep.equal([0, 68, 5, 82]);
  });

  it("handles partial line selection (starts mid-line)", () => {
    const chars = [
      ch("A", [0, 90, 5, 100], { inlineRect: [0, 88, 200, 102] }),
      ch("B", [5, 90, 10, 100], { inlineRect: [0, 88, 200, 102] }),
      ch("C", [10, 90, 15, 100], { inlineRect: [0, 88, 200, 102], lineBreakAfter: true }),
    ];
    // Select only B..C (indices 1..2)
    const rects = getRangeRects(chars, 1, 2);
    expect(rects).to.have.lengthOf(1);
    // x from B's rect[0]=5, y from B's inlineRect[1]=88, x2 from C's rect[2]=15, y2 from B's inlineRect[3]=102
    expect(rects[0]).to.deep.equal([5, 88, 15, 102]);
  });

  it("handles vertical text (rotation=90)", () => {
    const chars = [
      ch("A", [90, 0, 100, 5], { inlineRect: [88, 0, 102, 200], rotation: 90, lineBreakAfter: true }),
      ch("B", [90, 5, 100, 10], { inlineRect: [88, 0, 102, 200], rotation: 90, lineBreakAfter: true }),
    ];
    const rects = getRangeRects(chars, 0, 1);
    expect(rects).to.have.lengthOf(2);
    // For vertical: rect = [firstInline[0], firstRect[1], firstInline[2], lastRect[3]]
    expect(rects[0]).to.deep.equal([88, 0, 102, 5]);
    expect(rects[1]).to.deep.equal([88, 5, 102, 10]);
  });

  it("normalizes rects with inverted coordinates", () => {
    const chars = [
      ch("A", [15, 100, 0, 90], { inlineRect: [200, 102, 0, 88], lineBreakAfter: true }),
    ];
    const rects = getRangeRects(chars, 0, 0);
    expect(rects).to.have.lengthOf(1);
    // After normalization: rect = [0, 88, 15, 102] for horizontal
    // firstRect norm => [0, 90, 15, 100], firstInline norm => [0, 88, 200, 102]
    // horizontal: [firstRect[0], firstInline[1], lastRect[2], firstInline[3]] = [0, 88, 15, 102]
    expect(rects[0]).to.deep.equal([0, 88, 15, 102]);
  });

  it("falls back to char rect when inlineRect is absent", () => {
    const chars = [
      ch("A", [0, 90, 5, 100], { lineBreakAfter: true }),
    ];
    const rects = getRangeRects(chars, 0, 0);
    expect(rects).to.have.lengthOf(1);
    // inlineRect falls back to rect: [0, 90, 5, 100]
    // horizontal: [firstRect[0], firstInline[1], lastRect[2], firstInline[3]] = [0, 90, 5, 100]
    expect(rects[0]).to.deep.equal([0, 90, 5, 100]);
  });
});
