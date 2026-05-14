import { expect } from "chai";
import { validateAnnotationParams } from "../../src/utils/annotation";

describe("validateAnnotationParams", () => {
  const position = { pageIndex: 0, rects: [[0, 0, 10, 10]] };

  it("accepts highlight with text", () => {
    const r = validateAnnotationParams({
      type: "highlight", text: "hi", color: "#ffd400", position,
    });
    expect(r.ok).to.equal(true);
  });

  it("accepts underline with text", () => {
    const r = validateAnnotationParams({
      type: "underline", text: "hi", color: "#ffd400", position,
    });
    expect(r.ok).to.equal(true);
  });

  it("accepts image without text", () => {
    const r = validateAnnotationParams({
      type: "image", color: "#ffd400", position,
    });
    expect(r.ok).to.equal(true);
  });

  it("rejects image WITH text — text only valid for highlight/underline", () => {
    const r = validateAnnotationParams({
      type: "image", text: "nope", color: "#ffd400", position,
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/text.*highlight.*underline/i);
  });

  it("rejects 3-char hex color", () => {
    const r = validateAnnotationParams({
      type: "highlight", color: "#fff", position,
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/color/i);
  });

  it("rejects color without #", () => {
    const r = validateAnnotationParams({
      type: "highlight", color: "ffd400", position,
    });
    expect(r.ok).to.equal(false);
  });

  it("rejects unknown type", () => {
    const r = validateAnnotationParams({
      type: "scribble" as any, color: "#ffd400", position,
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/type/i);
  });

  it("accepts missing color (defaults applied later)", () => {
    const r = validateAnnotationParams({ type: "highlight", position });
    expect(r.ok).to.equal(true);
  });

  it("rejects missing or empty position", () => {
    for (const badPosition of [undefined, null, {}, [], { foo: 1 }, { pageIndex: 0 }, { pageIndex: 0, rects: [[0, 1, 2]] }]) {
      const r = validateAnnotationParams({
        type: "highlight",
        position: badPosition,
      } as any);
      expect(r.ok).to.equal(false);
      if (!r.ok) expect(r.message).to.match(/position/i);
    }
  });

  it("accepts ink position with paths", () => {
    const r = validateAnnotationParams({
      type: "ink",
      position: { pageIndex: 0, paths: [[[0, 0], [10, 10]]] },
    });
    expect(r.ok).to.equal(true);
  });

  it("rejects non-numeric sortIndex", () => {
    for (const badSortIndex of ["not-a-number", true, null, {}, []]) {
      const r = validateAnnotationParams({
        type: "highlight",
        position,
        sortIndex: badSortIndex,
      });
      expect(r.ok).to.equal(false);
      if (!r.ok) expect(r.message).to.match(/sortIndex/i);
    }
  });

  it("accepts Zotero PDF sortIndex strings", () => {
    const r = validateAnnotationParams({
      type: "highlight",
      position,
      sortIndex: "00000|000000|00165",
    });
    expect(r.ok).to.equal(true);
  });

  it("accepts highlight with quote and no position", () => {
    const r = validateAnnotationParams({
      type: "highlight",
      quote: "some text to find",
    });
    expect(r.ok).to.equal(true);
  });

  it("accepts underline with quote and no position", () => {
    const r = validateAnnotationParams({
      type: "underline",
      quote: "some text to find",
    });
    expect(r.ok).to.equal(true);
  });

  it("rejects quote for note type", () => {
    const r = validateAnnotationParams({
      type: "note",
      quote: "some text",
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/quote.*highlight.*underline/i);
  });

  it("rejects quote for image type", () => {
    const r = validateAnnotationParams({
      type: "image",
      quote: "some text",
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/quote.*highlight.*underline/i);
  });

  it("still requires position when quote is not provided", () => {
    const r = validateAnnotationParams({
      type: "highlight",
    });
    expect(r.ok).to.equal(false);
    if (!r.ok) expect(r.message).to.match(/position/i);
  });
});
