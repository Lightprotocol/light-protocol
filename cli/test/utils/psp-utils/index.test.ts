import { extractFilename } from "../../../src/psp-utils/utils";
import "mocha";
import { assert } from "chai";

describe("Testing the helper functions", () => {
  it("testing utils.extractFilename: correct input", () => {
    const input =
      "successfully created main tmpTestPspMain.circom and tmp_test_psp.circom";
    const filename = extractFilename(input);
    assert.isNotNull(filename);
    assert.equal(filename, "tmpTestPspMain.circom");
  });

  it("testing utils.extractFilename: empty input", () => {
    const input = "";
    const filename = extractFilename(input);
    assert.isNull(filename);
  });
});
