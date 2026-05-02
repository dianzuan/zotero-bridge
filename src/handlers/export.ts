// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/export.ts
import { registerHandlers } from "../server";
import { wrapTranslatorError } from "../utils/translator-error";
import { resolveItems } from "../utils/guards";

async function exportItems(idsOrKeys: (number | string)[], translatorID: string, format: string): Promise<string> {
  const items = await resolveItems(idsOrKeys);
  const translate = new Zotero.Translate.Export();
  translate.setItems(items);
  translate.setTranslator(translatorID);
  return new Promise((resolve, reject) => {
    translate.setHandler("done", (_obj: any, status: boolean) => {
      if (status) resolve(translate.string);
      else reject(wrapTranslatorError(format, new Error("translator returned failure status")));
    });
    translate.translate();
  });
}

// Known translator IDs (built into Zotero)
const TRANSLATORS = {
  bibtex: "9cb70025-a888-4a29-a210-93ec52da40d4",
  ris: "32d59d2d-b65a-4da4-b0a3-bdd3cfb979e7",
  cslJson: "bc03b4fe-436d-4a1f-ba59-de4d2d7a63f7",
  csv: "25f4c5e2-d790-4daa-a667-797619c7e2f2",
};

export const exportHandlers = {
  async bibtex(params: { ids: (number | string)[] }) {
    const output = await exportItems(params.ids, TRANSLATORS.bibtex, "bibtex");
    return { format: "bibtex", content: output, count: params.ids.length };
  },

  async cslJson(params: { ids: (number | string)[] }) {
    const output = await exportItems(params.ids, TRANSLATORS.cslJson, "cslJson");
    return { format: "csl-json", content: JSON.parse(output), count: params.ids.length };
  },

  async ris(params: { ids: (number | string)[] }) {
    const output = await exportItems(params.ids, TRANSLATORS.ris, "ris");
    return { format: "ris", content: output, count: params.ids.length };
  },

  async csv(params: { ids: (number | string)[]; fields?: string[] }) {
    const output = await exportItems(params.ids, TRANSLATORS.csv, "csv");
    return { format: "csv", content: output, count: params.ids.length };
  },

  async bibliography(params: { ids: (number | string)[]; style?: string }) {
    // We bypass Zotero.QuickCopy.getContentFromItems because in Zotero 8 its
    // internal path ended up calling style.getCiteProc on an object that
    // didn't have the method, surfacing as -32603. Going through
    // Zotero.Styles directly is the documented path and works across 6/7/8.
    const styleId = params.style
      || "http://www.zotero.org/styles/gb-t-7714-2015-numeric";
    const style = (Zotero as any).Styles.get(styleId);
    if (!style) {
      throw {
        code: -32602,
        message: `CSL style not installed: ${styleId}. ` +
                 `Install via Zotero → Settings → Cite → Styles.`,
      };
    }
    const itemIDs = params.ids;
    const engine = style.getCiteProc();
    const formats: { html: string; text: string } = { html: "", text: "" };
    try {
      for (const fmt of ["html", "text"] as const) {
        engine.setOutputFormat(fmt);
        engine.updateItems(itemIDs);
        const bib = engine.makeBibliography();
        // makeBibliography returns [metadata, entries[]] on success, or false.
        if (bib && Array.isArray(bib) && bib.length >= 2) {
          formats[fmt] = (bib[1] as string[]).join(fmt === "html" ? "" : "\n");
        }
      }
    } finally {
      engine.free?.();
    }
    return {
      format: "bibliography",
      style: styleId,
      html: formats.html,
      text: formats.text,
      count: itemIDs.length,
    };
  },

};

registerHandlers("export", exportHandlers);
