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

  const normalizedQuote = normalizeWs(quote);
  if (normalizedQuote.length === 0) return null;

  // Build the text string and a mapping from text positions back to char indices.
  // charIndexMap[textPos] = index into the chars array for that character.
  // Space characters inserted by spaceAfter/lineBreakAfter get -1 (synthetic).
  const textParts: string[] = [];
  const charIndexMap: number[] = [];

  for (let i = 0; i < chars.length; i++) {
    const char = chars[i];
    if (char.ignorable) continue;
    textParts.push(char.c);
    charIndexMap.push(i);
    if (char.spaceAfter || char.lineBreakAfter) {
      textParts.push(" ");
      charIndexMap.push(-1); // synthetic space
    }
  }

  const fullText = textParts.join("");
  const normalizedText = normalizeWs(fullText);

  // Find the quote in normalized text
  const matchPos = normalizedText.indexOf(normalizedQuote);
  if (matchPos === -1) return null;

  // Map normalized text positions back to original text positions.
  // normalizeWs trims leading whitespace and collapses runs, so we need
  // to walk both strings in sync.
  const origToNorm = buildOrigToNormMap(fullText);

  // Find the range in original text that corresponds to the normalized match
  let origStart = -1;
  let origEnd = -1;
  for (let i = 0; i < origToNorm.length; i++) {
    if (origToNorm[i] === matchPos && origStart === -1) {
      origStart = i;
    }
    if (origToNorm[i] === matchPos + normalizedQuote.length - 1) {
      origEnd = i;
    }
  }

  if (origStart === -1 || origEnd === -1) return null;

  // Map from original text positions to char indices, skipping synthetic spaces
  let charStart = -1;
  let charEnd = -1;

  for (let i = origStart; i >= 0; i--) {
    if (charIndexMap[i] !== -1) {
      charStart = charIndexMap[i];
      break;
    }
  }

  for (let i = origEnd; i >= 0; i--) {
    if (charIndexMap[i] !== -1) {
      charEnd = charIndexMap[i];
      break;
    }
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

// --- internal helpers ---

function normalizeWs(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}

/**
 * Build a mapping from each position in the original string to its
 * corresponding position in the normalized (whitespace-collapsed, trimmed) string.
 */
function buildOrigToNormMap(original: string): number[] {
  const map: number[] = [];
  const normalized = normalizeWs(original);

  let normIdx = 0;
  let inLeadingSpace = true;
  let prevWasSpace = false;

  for (let i = 0; i < original.length; i++) {
    const c = original[i];
    const isSpace = /\s/.test(c);

    if (inLeadingSpace) {
      if (isSpace) {
        map.push(-1);
        continue;
      }
      inLeadingSpace = false;
    }

    if (isSpace) {
      if (!prevWasSpace) {
        // This is the first space in a run — check if we're at trailing space
        if (normIdx < normalized.length && normalized[normIdx] === " ") {
          map.push(normIdx);
          normIdx++;
        } else {
          map.push(-1); // trailing space
        }
      } else {
        map.push(-1); // collapsed space
      }
      prevWasSpace = true;
    } else {
      map.push(normIdx);
      normIdx++;
      prevWasSpace = false;
    }
  }

  return map;
}
