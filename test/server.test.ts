import { expect } from "chai";
import { installZotero, resetZotero } from "./fixtures/zotero-mock";

describe("processRequest fuzzy suggestions", () => {
  beforeEach(() => {
    // Clear require caches for fresh handler registration
    delete require.cache[require.resolve("../src/server")];
    delete require.cache[require.resolve("../src/handlers/notes")];
  });
  afterEach(() => resetZotero());

  it("suggests closest method when calling unknown method", async () => {
    installZotero({});
    const { createEndpointHandler } = await import("../src/server");
    await import("../src/handlers/notes");

    const Handler = createEndpointHandler();
    const h = new (Handler as any)();
    const [status, , body] = await h.init({
      jsonrpc: "2.0", id: 1, method: "notes.creates", params: {},
    });
    const parsed = JSON.parse(body);
    expect(parsed.error.code).to.equal(-32601);
    expect(parsed.error.message).to.contain("Did you mean");
    expect(parsed.error.message).to.contain("notes.create");
  });

  it("does NOT suggest when distance is too large", async () => {
    installZotero({});
    const { createEndpointHandler } = await import("../src/server");
    await import("../src/handlers/notes");

    const Handler = createEndpointHandler();
    const h = new (Handler as any)();
    const [status, , body] = await h.init({
      jsonrpc: "2.0", id: 1, method: "completely.wrong.method.name.xyz", params: {},
    });
    const parsed = JSON.parse(body);
    expect(parsed.error.code).to.equal(-32601);
    expect(parsed.error.message).to.not.contain("Did you mean");
  });
});
