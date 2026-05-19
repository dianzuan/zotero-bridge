import { expect } from "chai";
import sinon from "sinon";

describe("pdf-extract", () => {
  let pdfExtract: typeof import("../../src/utils/pdf-extract");

  beforeEach(async () => {
    delete require.cache[require.resolve("../../src/utils/pdf-extract")];
    pdfExtract = await import("../../src/utils/pdf-extract");
  });

  afterEach(() => {
    pdfExtract.__test__.reset();
    sinon.restore();
  });

  it("exports extractPageChars, getPageCount, shutdown, and __test__", () => {
    expect(pdfExtract.extractPageChars).to.be.a("function");
    expect(pdfExtract.getPageCount).to.be.a("function");
    expect(pdfExtract.shutdown).to.be.a("function");
    expect(pdfExtract.__test__).to.be.an("object");
  });

  it("extractPageChars delegates to the injected implementation", async () => {
    const fakeChars = [
      { c: "A", rect: [0, 0, 5, 10] as [number, number, number, number] },
      { c: "B", rect: [5, 0, 10, 10] as [number, number, number, number] },
    ];
    const stub = sinon.stub().resolves(fakeChars);
    pdfExtract.__test__.setExtractImpl(stub);

    const fakeAttachment = { id: 42 } as any;
    const result = await pdfExtract.extractPageChars(fakeAttachment, 3);

    expect(stub.calledOnce).to.be.true;
    expect(stub.firstCall.args[0]).to.equal(fakeAttachment);
    expect(stub.firstCall.args[1]).to.equal(3);
    expect(result).to.deep.equal(fakeChars);
  });

  it("getPageCount delegates to the injected implementation", async () => {
    const stub = sinon.stub().resolves(21);
    pdfExtract.__test__.setCountImpl(stub);

    const fakeAttachment = { id: 99 } as any;
    const result = await pdfExtract.getPageCount(fakeAttachment);

    expect(stub.calledOnce).to.be.true;
    expect(result).to.equal(21);
  });
});
