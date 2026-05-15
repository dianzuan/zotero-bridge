// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill

/**
 * Pure functions for locating text quotes using Zotero reader's per-char data.
 * No Zotero global dependency — fully unit-testable.
 *
 * These operate on the `chars` array from:
 *   reader._internalReader._primaryView._pdfPages[pageIndex].chars
 * where each char has { c, rect, spaceAfter, lineBreakAfter, ignorable, inlineRect, rotation }.
 */

export interface CharData {
  c: string;
  rect: [number, number, number, number];
  spaceAfter?: boolean;
  lineBreakAfter?: boolean;
  ignorable?: boolean;
  inlineRect?: [number, number, number, number];
  rotation?: number;
}

export interface QuoteMatch {
  offsetStart: number;  // index into the chars array (non-ignorable mapping)
  offsetEnd: number;    // index into the chars array (inclusive)
}

/**
 * Build a searchable text string from the reader's chars array.
 * Mirrors Zotero reader's own `getTextFromChars`.
 */
export function getTextFromChars(chars: CharData[]): string {
  const text: string[] = [];
  for (const char of chars) {
    if (!char.ignorable) {
      text.push(char.c);
      if (char.spaceAfter || char.lineBreakAfter) {
        text.push(" ");
      }
    }
  }
  return text.join("");
}

/**
 * Find a quote substring within the chars-derived text.
 * Whitespace is normalized in both the chars-text and the quote.
 *
 * Returns char-array indices (offsetStart, offsetEnd inclusive) or null.
 */
export function findQuoteInChars(chars: CharData[], quote: string): QuoteMatch | null {
  if (!quote || quote.trim().length === 0) return null;
  if (chars.length === 0) return null;

  // Build text from chars with a position map back to char indices.
  // charIndexMap[textPos] = index into chars array (-1 for synthetic spaces).
  const textParts: string[] = [];
  const charIndexMap: number[] = [];

  for (let i = 0; i < chars.length; i++) {
    const char = chars[i];
    if (char.ignorable) continue;
    textParts.push(char.c);
    charIndexMap.push(i);
    if (char.spaceAfter || char.lineBreakAfter) {
      textParts.push(" ");
      charIndexMap.push(-1);
    }
  }

  const fullText = textParts.join("");

  // Strip ALL whitespace for matching — handles cross-line CJK (where line
  // breaks insert spaces into continuous text) and English word boundaries.
  const strippedQuote = quote.replace(/\s/g, "");
  if (strippedQuote.length === 0) return null;

  const strippedText = fullText.replace(/\s/g, "");
  const matchPos = strippedText.indexOf(strippedQuote);
  if (matchPos === -1) return null;

  // Map stripped-text positions back to fullText positions.
  const strippedToFull: number[] = [];
  for (let i = 0; i < fullText.length; i++) {
    if (!/\s/.test(fullText[i])) strippedToFull.push(i);
  }

  const fullStart = strippedToFull[matchPos];
  const fullEnd = strippedToFull[matchPos + strippedQuote.length - 1];

  // Map fullText positions to char indices, skipping synthetic spaces.
  let charStart = -1;
  let charEnd = -1;
  for (let i = fullStart; i >= 0; i--) {
    if (charIndexMap[i] !== -1) { charStart = charIndexMap[i]; break; }
  }
  for (let i = fullEnd; i >= 0; i--) {
    if (charIndexMap[i] !== -1) { charEnd = charIndexMap[i]; break; }
  }

  if (charStart === -1 || charEnd === -1) return null;
  return { offsetStart: charStart, offsetEnd: charEnd };
}

/**
 * Build line-level rects from a range of chars.
 * Mirrors Zotero reader's own `getRangeRects`.
 */
export function getRangeRects(
  chars: CharData[],
  offsetStart: number,
  offsetEnd: number,
): number[][] {
  const rects: number[][] = [];
  let start = offsetStart;

  const norm = (r: [number, number, number, number]): [number, number, number, number] => {
    const [x1, y1, x2, y2] = r;
    return [Math.min(x1, x2), Math.min(y1, y2), Math.max(x1, x2), Math.max(y1, y2)];
  };

  for (let i = start; i <= offsetEnd; i++) {
    const char = chars[i];
    const isBreak = char.lineBreakAfter || i === offsetEnd;
    if (!isBreak) continue;

    const firstChar = chars[start];
    const lastChar = char;
    const firstRect = norm(firstChar.rect);
    const lastRect = norm(lastChar.rect);
    const firstInline = norm((firstChar.inlineRect || firstChar.rect) as [number, number, number, number]);
    const rot = firstChar?.rotation ?? 0;
    const isVertical = rot === 90 || rot === 270;

    let rect: number[];
    if (isVertical) {
      rect = [firstInline[0], firstRect[1], firstInline[2], lastRect[3]];
    } else {
      rect = [firstRect[0], firstInline[1], lastRect[2], firstInline[3]];
    }
    rects.push(rect);
    start = i + 1;
  }

  return rects;
}

/**
 * Convert PDFWorker.getRecognizerData page data into a flat CharData array.
 * Used by system.pdfRecognizerData diagnostic RPC. Not used for annotation
 * creation (which uses background reader for correct coordinates).
 */
export function recognizerPageToChars(pageData: any): CharData[] {
  const pageHeight = pageData?.[1] ?? 0;
  const lineGroups = pageData?.[2];
  if (!Array.isArray(lineGroups)) return [];

  const allChars: CharData[] = [];
  const rawChars: { c: string; rect: [number, number, number, number]; spaceAfter: boolean; rotation: number; baseline: number }[] = [];

  // Recursively collect all char arrays (14+ elements, last is string)
  function collect(node: any): void {
    if (!Array.isArray(node)) return;
    if (node.length >= 14 && typeof node[13] === "string") {
      // PDFWorker uses top-left origin; Zotero annotations use bottom-left.
      // Flip Y: y_zotero = pageHeight - y_topLeft
      const yMin = node[1];
      const yMax = node[3];
      rawChars.push({
        c: node[13],
        rect: [node[0], pageHeight - yMax, node[2], pageHeight - yMin],
        spaceAfter: !!node[5],
        rotation: node[7] || 0,
        baseline: pageHeight - (node[6] || 0),
      });
      return;
    }
    for (const child of node) collect(child);
  }
  collect(lineGroups);

  // Detect line breaks: when baseline changes significantly between consecutive chars
  for (let i = 0; i < rawChars.length; i++) {
    const rc = rawChars[i];
    const next = rawChars[i + 1];
    const lineBreak = next ? Math.abs(rc.baseline - next.baseline) > rc.rect[3] - rc.rect[1] : false;
    allChars.push({
      c: rc.c,
      rect: rc.rect,
      spaceAfter: rc.spaceAfter,
      lineBreakAfter: lineBreak,
      rotation: rc.rotation,
    });
  }

  return allChars;
}

