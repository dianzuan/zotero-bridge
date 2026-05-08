// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
/**
 * Extract the year from a Zotero item's `date` field via Zotero.Date.strToDate.
 *
 * Replaces the broken `item.getField("year")` idiom — "year" is not a
 * primary Zotero field and only resolves on certain item types. The
 * `date` field is universal and `strToDate` parses ISO/freeform dates
 * into {year, month, day}.
 *
 * Returns the year as a number, or "" when the date is missing or unparseable.
 */
export function extractYear(item: { getField: (name: string) => string }): number | "" {
  const dateStr = item.getField("date");
  if (!dateStr) return "";
  const parsed = Zotero.Date.strToDate(dateStr);
  return typeof parsed?.year === "number" ? parsed.year : "";
}

export function creatorKeyName(creator: { firstName?: string; lastName?: string; name?: string } | undefined): string {
  if (!creator) return "Unknown";
  if (creator.name) return creator.name.replace(/\s+/g, "");
  const firstName = creator.firstName || "";
  const lastName = creator.lastName || "";
  const combined = `${lastName}${firstName}`;
  if (/[\u3400-\u9fff]/.test(combined)) return combined || "Unknown";
  return [lastName, firstName].filter(Boolean).join("").replace(/\s+/g, "") || "Unknown";
}
