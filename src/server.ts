// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
type HandlerFn = (params: any) => Promise<any>;
type HandlerMap = Record<string, HandlerFn>;
const handlers: HandlerMap = {};

export function registerHandlers(namespace: string, methods: Record<string, HandlerFn>) {
  for (const [name, fn] of Object.entries(methods)) {
    handlers[`${namespace}.${name}`] = fn;
  }
}

const INVALID_REQUEST = -32600;
const METHOD_NOT_FOUND = -32601;
const INTERNAL_ERROR = -32603;

function levenshtein(a: string, b: string): number {
  const m = a.length, n = b.length;
  const dp = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = a[i - 1] === b[j - 1]
        ? dp[i - 1][j - 1]
        : 1 + Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
    }
  }
  return dp[m][n];
}

function findClosestMethod(input: string): string | null {
  const methods = Object.keys(handlers);
  let best = "";
  let bestDist = Infinity;
  for (const m of methods) {
    const d = levenshtein(input.toLowerCase(), m.toLowerCase());
    if (d < bestDist) { bestDist = d; best = m; }
  }
  return bestDist <= 5 ? best : null;
}

function jsonRpcError(id: any, code: number, message: string) {
  return JSON.stringify({ jsonrpc: "2.0", error: { code, message }, id });
}
function jsonRpcResult(id: any, result: any) {
  return JSON.stringify({ jsonrpc: "2.0", result, id });
}

async function processRequest(req: any): Promise<string> {
  if (!req || req.jsonrpc !== "2.0" || !req.method) {
    return jsonRpcError(req?.id ?? null, INVALID_REQUEST, "Invalid JSON-RPC 2.0 request");
  }
  const handler = handlers[req.method];
  if (!handler) {
    const suggestion = findClosestMethod(req.method);
    const msg = suggestion
      ? `Method not found: ${req.method}. Did you mean ${suggestion}?`
      : `Method not found: ${req.method}`;
    return jsonRpcError(req.id, METHOD_NOT_FOUND, msg);
  }
  try {
    const result = await handler(req.params || {});
    return jsonRpcResult(req.id, result);
  } catch (err: any) {
    return jsonRpcError(req.id, err.code || INTERNAL_ERROR, err.message || "Internal error");
  }
}

export function createEndpointHandler() {
  const Handler = function () {};
  Handler.prototype = {
    supportedMethods: ["POST"],
    supportedDataTypes: ["application/json", "text/plain"],
    permitBookmarklet: false,
    async init(request: any) {
      // Zotero HTTP server passes a request object with .data (parsed JSON)
      const parsed = request.data || request;

      if (Array.isArray(parsed)) {
        const results = await Promise.all(parsed.map(processRequest));
        return [200, "application/json", `[${results.join(",")}]`];
      }
      const result = await processRequest(parsed);
      return [200, "application/json", result];
    },
  };
  return Handler;
}

export function registerEndpoint() {
  Zotero.Server.Endpoints["/zotron/rpc"] = createEndpointHandler();
  Zotero.log("[Zotron] JSON-RPC endpoint registered at /zotron/rpc");
}
export function unregisterEndpoint() {
  delete Zotero.Server.Endpoints["/zotron/rpc"];
}
export function getRegisteredMethods(): string[] {
  return Object.keys(handlers).sort();
}
