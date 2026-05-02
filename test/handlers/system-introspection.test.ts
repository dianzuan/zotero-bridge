import { expect } from "chai";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("system introspection", () => {
  beforeEach(() => {
    // Clear all handler modules so registered methods are fresh
    delete require.cache[require.resolve("../../src/server")];
    delete require.cache[require.resolve("../../src/handlers/system")];
  });
  afterEach(() => resetZotero());

  it("system.listMethods returns sorted array of method names", async () => {
    installZotero({});
    await import("../../src/handlers/system");
    const { systemHandlers } = await import("../../src/handlers/system");
    const result = await systemHandlers.listMethods();
    expect(result).to.be.an("array");
    expect(result).to.include("system.listMethods");
    expect(result).to.include("system.ping");
    // Verify sorted
    const sorted = [...result].sort();
    expect(result).to.deep.equal(sorted);
  });

  it("system.describe without args returns all method schemas", async () => {
    installZotero({});
    await import("../../src/handlers/system");
    const { systemHandlers } = await import("../../src/handlers/system");
    const result = await systemHandlers.describe({});
    expect(result).to.be.an("array");
    expect(result.length).to.be.greaterThan(0);
    expect(result[0]).to.have.property("name");
  });

  it("system.describe with method name returns single schema", async () => {
    installZotero({});
    await import("../../src/handlers/system");
    const { systemHandlers } = await import("../../src/handlers/system");
    const result = await systemHandlers.describe({ method: "system.ping" });
    expect(result.name).to.equal("system.ping");
  });

  it("system.describe with unknown method throws -32601", async () => {
    installZotero({});
    await import("../../src/handlers/system");
    const { systemHandlers } = await import("../../src/handlers/system");
    try {
      await systemHandlers.describe({ method: "nonexistent.method" });
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.code).to.equal(-32601);
    }
  });
});
