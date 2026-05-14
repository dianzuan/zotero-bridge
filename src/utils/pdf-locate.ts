// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill

/**
 * Pure functions for locating text quotes in PDF.js text layer output.
 * No Zotero global dependency — fully unit-testable.
 */

export interface TextItem {
  str: string;
  transform: number[];
  width: number;
  height: number;
}

export interface TextLine {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface LocateResult {
  rects: number[][];
  matchedText: string;
}

/**
 * Merge PDF.js getTextContent().items into lines sharing a baseline.
 *
 * Items whose vertical position overlaps (accounting for super/subscript)
 * are merged into a single line. Whitespace-only items are filtered out.
 */
export function mergeTextItems(items: TextItem[]): TextLine[] {
  const filtered = items.filter((item) => item.str.trim().length > 0);
  if (filtered.length === 0) return [];

  function toLine(item: TextItem): TextLine & { _heights: number[] } {
    let x = parseFloat(item.transform[4].toFixed(1));
    let width = item.width;
    if (width < 0) {
      x += width;
      width = -width;
    }
    return {
      x,
      y: parseFloat(item.transform[5].toFixed(1)),
      text: item.str,
      height: item.height,
      width,
      _heights: [item.height],
    };
  }

  const lines: (TextLine & { _heights: number[] })[] = [toLine(filtered[0])];

  for (let i = 1; i < filtered.length; i++) {
    const cur = toLine(filtered[i]);
    const last = lines[lines.length - 1];

    const sameBaseline =
      cur.y === last.y ||
      (cur.y >= last.y && cur.y < last.y + last.height) ||
      (cur.y + cur.height > last.y && cur.y + cur.height <= last.y + last.height);

    if (sameBaseline) {
      last.text += " " + cur.text;
      last.width += cur.width;
      last._heights.push(cur.height);
    } else {
      // Finalize previous line height using the mode
      last.height = modeHeight(last._heights);
      lines.push(cur);
    }
  }

  // Finalize last line
  const last = lines[lines.length - 1];
  last.height = modeHeight(last._heights);

  return lines.map(({ _heights, ...line }) => line);
}

function modeHeight(heights: number[]): number {
  const freq: Record<string, number> = {};
  for (const h of heights) {
    const key = String(h);
    freq[key] = (freq[key] ?? 0) + 1;
  }
  return Number(
    Object.keys(freq).sort((a, b) => freq[b] - freq[a])[0],
  );
}

/**
 * Locate a quote string within merged page lines.
 *
 * Whitespace is normalized for matching: runs of spaces in both the page
 * text and the quote are collapsed to single spaces. The quote may span
 * multiple lines.
 *
 * Returns one [x1, y1, x2, y2] rect per line the quote spans, or null
 * if the quote is not found.
 */
export function locateQuote(pageLines: TextLine[], quote: string): LocateResult | null {
  if (!quote || quote.trim().length === 0) return null;
  if (pageLines.length === 0) return null;

  const normalizedQuote = normalizeWs(quote);

  // Build concatenated text from all lines with line boundaries tracked
  const segments: { lineIdx: number; startInConcat: number; text: string }[] = [];
  let concat = "";
  for (let i = 0; i < pageLines.length; i++) {
    const text = normalizeWs(pageLines[i].text);
    if (concat.length > 0) concat += " ";
    segments.push({ lineIdx: i, startInConcat: concat.length, text });
    concat += text;
  }

  const matchStart = concat.indexOf(normalizedQuote);
  if (matchStart === -1) return null;
  const matchEnd = matchStart + normalizedQuote.length;

  const rects: number[][] = [];
  const matchedText = concat.slice(matchStart, matchEnd);

  for (const seg of segments) {
    const segStart = seg.startInConcat;
    const segEnd = segStart + seg.text.length;

    // Does this segment overlap with the match range?
    const overlapStart = Math.max(matchStart, segStart);
    const overlapEnd = Math.min(matchEnd, segEnd);
    if (overlapStart >= overlapEnd) continue;

    const line = pageLines[seg.lineIdx];
    const charTotal = seg.text.length;

    // Character-proportional x offset within the line
    const fracStart = (overlapStart - segStart) / charTotal;
    const fracEnd = (overlapEnd - segStart) / charTotal;

    const x1 = line.x + fracStart * line.width;
    const x2 = line.x + fracEnd * line.width;
    const y1 = line.y;
    const y2 = line.y + line.height;

    rects.push([x1, y1, x2, y2]);
  }

  return rects.length > 0 ? { rects, matchedText } : null;
}

function normalizeWs(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}
