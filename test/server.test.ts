import { expect } from "chai";
import { installZotero, resetZotero } from "./fixtures/zotero-mock";
import { registerHandlers, createEndpointHandler } from "../src/server";

describe("processRequest fuzzy suggestions", () => {
  before(() => {
    registerHandlers("fake", {
      create: async () => ({ ok: true }),
      search: async () => [],
    });
  });
  afterEach(() => resetZotero());

  it("suggests closest method when calling unknown method", async () => {
    installZotero({});
    const Handler = createEndpointHandler();
    const h = new (Handler as any)();
    const [, , body] = await h.init({
      jsonrpc: "2.0", id: 1, method: "fake.creates", params: {},
    });
    const parsed = JSON.parse(body);
    expect(parsed.error.code).to.equal(-32601);
    expect(parsed.error.message).to.contain("Did you mean");
    expect(parsed.error.message).to.contain("fake.create");
  });

  it("does NOT suggest when distance is too large", async () => {
    installZotero({});
    const Handler = createEndpointHandler();
    const h = new (Handler as any)();
    const [, , body] = await h.init({
      jsonrpc: "2.0", id: 1, method: "completely.wrong.method.name.xyz", params: {},
    });
    const parsed = JSON.parse(body);
    expect(parsed.error.code).to.equal(-32601);
    expect(parsed.error.message).to.not.contain("Did you mean");
  });
});
