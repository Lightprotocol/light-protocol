import { extractFilename } from "../../../src/psp-utils/utils";
import "mocha";
import { assert } from "chai";

describe("Testing the helper functions", () => {
  it("testing utils.extractFilename: correct input", () => {
    let input =
      "sucessfully created main tmpTestPspMain.circom and tmp_test_psp.circom";
    let filename = extractFilename(input);
    assert.isNotNull(filename);
    assert.equal(filename, "tmpTestPspMain.circom");
  });

  it("testing utils.extractFilename: empty input", () => {
    let input = "";
    let filename = extractFilename(input);
    assert.isNull(filename);
  });
});
