// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
export type AnnotationType = "highlight" | "underline" | "note" | "image" | "ink";

const VALID_TYPES: ReadonlySet<AnnotationType> = new Set([
  "highlight", "underline", "note", "image", "ink",
]);

const TEXT_TYPES: ReadonlySet<AnnotationType> = new Set(["highlight", "underline"]);

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;
const PDF_SORT_INDEX = /^\d{5}\|\d{6}\|\d{5}$/;

export interface AnnotationParams {
  type: AnnotationType;
  text?: string;
  color?: string;
  comment?: string;
  position?: any;
  quote?: string;
  sortIndex?: unknown;
}

export type ValidationResult =
  | { ok: true }
  | { ok: false; message: string };

/**
 * Validate annotation params against Zotero 8 invariants:
 * - type must be one of the 5 known annotation types
 * - text is only valid for highlight/underline (item.js#L4243)
 * - color must be #RRGGBB hex (item.js#L4249)
 *
 * Returns {ok:true} on success, {ok:false, message} on failure.
 * The handler maps {ok:false} to throw {code: -32602, message}.
 */
export function validateAnnotationParams(p: AnnotationParams): ValidationResult {
  if (!VALID_TYPES.has(p.type)) {
    return {
      ok: false,
      message: `Invalid annotation type: ${p.type} (expected one of: ${[...VALID_TYPES].join(", ")})`,
    };
  }
  if (p.text !== undefined && p.text !== "" && !TEXT_TYPES.has(p.type)) {
    return {
      ok: false,
      message: `annotationText is only valid for highlight or underline annotations (got type=${p.type})`,
    };
  }
  if (p.color !== undefined && !HEX_COLOR.test(p.color)) {
    return {
      ok: false,
      message: `Invalid annotation color: ${p.color} (expected #RRGGBB 6-char hex)`,
    };
  }
  if (p.quote !== undefined && p.quote !== "") {
    if (!TEXT_TYPES.has(p.type)) {
      return {
        ok: false,
        message: `quote-based locate is only valid for highlight or underline annotations (got type=${p.type})`,
      };
    }
    // In quote mode, position is resolved by the handler — skip position validation
    if (p.position === undefined || p.position === null) {
      return { ok: true };
    }
  }
  if (
    p.position === undefined
    || p.position === null
    || Array.isArray(p.position)
    || typeof p.position !== "object"
    || Object.keys(p.position).length === 0
  ) {
    return {
      ok: false,
      message: "annotation position must be a non-empty object",
    };
  }
  const positionResult = validateAnnotationPosition(p.type, p.position);
  if (!positionResult.ok) {
    return positionResult;
  }
  try {
    JSON.stringify(p.position);
  } catch (err: any) {
    return {
      ok: false,
      message: `annotation position must be JSON-serializable: ${err.message}`,
    };
  }
  if (p.sortIndex !== undefined) {
    if (typeof p.sortIndex !== "number" && typeof p.sortIndex !== "string") {
      return {
        ok: false,
        message: `annotation sortIndex must be a Zotero PDF sort index or numeric y-offset (got ${String(p.sortIndex)})`,
      };
    }
    const raw = typeof p.sortIndex === "number" ? String(p.sortIndex) : p.sortIndex.trim();
    if (!PDF_SORT_INDEX.test(raw) && !Number.isFinite(Number(raw))) {
      return {
        ok: false,
        message: `annotation sortIndex must be a Zotero PDF sort index or numeric y-offset (got ${p.sortIndex})`,
      };
    }
  }
  return { ok: true };
}

function validateAnnotationPosition(type: AnnotationType, position: any): ValidationResult {
  if (!Number.isInteger(position.pageIndex) || position.pageIndex < 0) {
    return {
      ok: false,
      message: "annotation position must include a non-negative integer pageIndex",
    };
  }

  if (type === "ink") {
    if (!Array.isArray(position.paths) || position.paths.length === 0) {
      return {
        ok: false,
        message: "ink annotation position must include non-empty paths",
      };
    }
    return { ok: true };
  }

  if (!Array.isArray(position.rects) || position.rects.length === 0 || !position.rects.every(isRect)) {
    return {
      ok: false,
      message: "annotation position must include non-empty rects of [x1, y1, x2, y2]",
    };
  }
  return { ok: true };
}

function isRect(value: any): boolean {
  return Array.isArray(value)
    && value.length === 4
    && value.every((coord) => typeof coord === "number" && Number.isFinite(coord));
}
