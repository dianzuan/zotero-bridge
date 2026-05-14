import { expect } from "chai";
import { mergeTextItems, locateQuote } from "../../src/utils/pdf-locate";

describe("mergeTextItems", () => {
  it("merges items sharing a baseline into a single line", () => {
    const items = [
      { str: "Hello", transform: [1, 0, 0, 1, 10, 100], width: 30, height: 12 },
      { str: "world", transform: [1, 0, 0, 1, 42, 100], width: 28, height: 12 },
    ];
    const lines = mergeTextItems(items);
    expect(lines).to.have.lengthOf(1);
    expect(lines[0].text).to.include("Hello");
    expect(lines[0].text).to.include("world");
    expect(lines[0].y).to.equal(100);
  });

  it("separates items on different baselines into separate lines", () => {
    const items = [
      { str: "Line one", transform: [1, 0, 0, 1, 10, 200], width: 50, height: 12 },
      { str: "Line two", transform: [1, 0, 0, 1, 10, 180], width: 50, height: 12 },
    ];
    const lines = mergeTextItems(items);
    expect(lines).to.have.lengthOf(2);
    expect(lines[0].text).to.include("Line one");
    expect(lines[1].text).to.include("Line two");
  });

  it("handles superscript/subscript by merging overlapping baselines", () => {
    const items = [
      { str: "Main", transform: [1, 0, 0, 1, 10, 100], width: 30, height: 12 },
      { str: "sup", transform: [1, 0, 0, 1, 42, 106], width: 15, height: 8 },
    ];
    const lines = mergeTextItems(items);
    expect(lines).to.have.lengthOf(1);
    expect(lines[0].text).to.include("Main");
    expect(lines[0].text).to.include("sup");
  });

  it("returns empty array for empty input", () => {
    expect(mergeTextItems([])).to.deep.equal([]);
  });

  it("handles items with negative width by flipping x", () => {
    const items = [
      { str: "RTL", transform: [1, 0, 0, 1, 50, 100], width: -30, height: 12 },
    ];
    const lines = mergeTextItems(items);
    expect(lines).to.have.lengthOf(1);
    expect(lines[0].x).to.equal(20);
    expect(lines[0].width).to.equal(30);
  });

  it("filters out whitespace-only items", () => {
    const items = [
      { str: "Hello", transform: [1, 0, 0, 1, 10, 100], width: 30, height: 12 },
      { str: "   ", transform: [1, 0, 0, 1, 42, 100], width: 10, height: 12 },
      { str: "world", transform: [1, 0, 0, 1, 54, 100], width: 28, height: 12 },
    ];
    const lines = mergeTextItems(items);
    expect(lines).to.have.lengthOf(1);
    expect(lines[0].text).to.include("Hello");
    expect(lines[0].text).to.include("world");
  });
});

describe("locateQuote", () => {
  function makeLine(text: string, x: number, y: number, width: number, height: number) {
    return { text, x, y, width, height };
  }

  const pageLines = [
    makeLine("This is the first line of the document.", 10, 700, 300, 12),
    makeLine("The second line continues the paragraph here.", 10, 685, 340, 12),
    makeLine("A third line wraps up the content nicely.", 10, 670, 320, 12),
  ];

  it("finds a single-line quote and returns one rect", () => {
    const result = locateQuote(pageLines, "first line of the document");
    expect(result).to.not.be.null;
    expect(result!.rects).to.have.lengthOf(1);
    expect(result!.matchedText).to.include("first line of the document");
    const rect = result!.rects[0];
    expect(rect).to.have.lengthOf(4);
    // rect is [x1, y1, x2, y2] within the first line
    expect(rect[1]).to.equal(700);
    expect(rect[3]).to.equal(700 + 12);
  });

  it("finds a multi-line quote and returns one rect per line", () => {
    const result = locateQuote(pageLines, "paragraph here. A third line wraps");
    expect(result).to.not.be.null;
    expect(result!.rects).to.have.lengthOf(2);
    // first rect on line 2 (y=685), second on line 3 (y=670)
    expect(result!.rects[0][1]).to.equal(685);
    expect(result!.rects[1][1]).to.equal(670);
  });

  it("matches whitespace-insensitively (collapsed spaces)", () => {
    const result = locateQuote(pageLines, "first  line   of  the  document");
    expect(result).to.not.be.null;
    expect(result!.rects).to.have.lengthOf(1);
  });

  it("returns null when quote is not found", () => {
    const result = locateQuote(pageLines, "this text does not exist anywhere");
    expect(result).to.be.null;
  });

  it("returns null for empty quote", () => {
    const result = locateQuote(pageLines, "");
    expect(result).to.be.null;
  });

  it("returns null for empty page lines", () => {
    const result = locateQuote([], "some text");
    expect(result).to.be.null;
  });

  it("handles a quote spanning all lines", () => {
    const result = locateQuote(
      pageLines,
      "first line of the document. The second line continues the paragraph here. A third line",
    );
    expect(result).to.not.be.null;
    expect(result!.rects).to.have.lengthOf(3);
  });

  it("computes sub-line x offsets for partial matches", () => {
    const result = locateQuote(pageLines, "second line");
    expect(result).to.not.be.null;
    const rect = result!.rects[0];
    // The "second line" starts partway into the second line text
    // x1 should be > line start x (10), x2 should be < line end
    expect(rect[0]).to.be.greaterThan(10);
    expect(rect[2]).to.be.lessThan(10 + 340);
  });
});
