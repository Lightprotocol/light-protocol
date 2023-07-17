import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
chai.use(chaiAsPromised);
import { it } from "mocha";
const fs = require("fs");

import { createVerifyingkeyRsFile } from "../src/utils";

describe("Test createVerifyingkeyRsFile Functional", () => {
  let readFileStub: any;
  let writeFileStub: any;
  let closeStub: any;

  beforeEach(function () {
    // Stub the fs.readFile method
    readFileStub = fs.readFile;
    fs.readFile = function (
      path: any,
      options: any,
      callback: (arg0: null, arg1: string) => void
    ) {
      const verifyingKeyData = {
        vk_alpha_1: { a: "123", b: "456" },
        vk_beta_2: { a: ["789", "012"], b: ["345", "678"] },
        vk_gamma_2: { a: ["901", "234"], b: ["567", "890"] },
        vk_delta_2: { a: ["123", "456"], b: ["789", "012"] },
        vk_alphabeta_12: { a: [["345", "678"]], b: [["901", "234"]] },
        IC: [["567", "890"]],
      };
      callback(null, JSON.stringify(verifyingKeyData));
    };

    // Stub the fs.writeFile method
    writeFileStub = fs.writeFile;
    fs.writeFile = function (
      path: any,
      data: string,
      options: any,
      callback: (arg0: null) => void
    ) {
      // Verify the content of the verifying_key.rs file
      const expectedFileContent = `use groth16_solana::groth16::Groth16Verifyingkey;
      use anchor_lang::prelude::*;

      pub const VERIFYINGKEY: Groth16Verifyingkey = Groth16Verifyingkey {
      \tnr_pubinputs: 1,
      \tvk_alpha_g1: [
      \t\t[123, 456],
      \t],
      \tvk_beta_g2: [
      \t\t[[789, 012], [345, 678]],
      \t],
      \tvk_gamma_g2: [
      \t\t[[901, 234], [567, 890]],
      \t],
      \tvk_delta_g2: [
      \t\t[[123, 456], [789, 012]],
      \t],
      \tvk_alphabeta_12: [
      \t\t[[[345, 678]]],
      \t],
      \tvk_ic: &[
      \t\t[[567, 890]],
      \t]
      };`;
      assert.equal(data.trim(), expectedFileContent.trim());
      callback(null);
    };

    // Stub the fs.close method
    closeStub = fs.close;
    fs.close = function (fd: any, callback: (arg0: null) => void) {
      callback(null);
    };
  });

  afterEach(function () {
    // Restore the original fs methods
    fs.readFile = readFileStub;
    fs.writeFile = writeFileStub;
    fs.close = closeStub;
  });

  it("should write the verifying_key.rs file with the correct content", async function () {
    const program = "???";
    const paths = ["???"];
    const vKeyJsonPath = "???";
    const vKeyRsPath = "???";
    const circuitName = "???";
    const artifiactPath = "???";

    await createVerifyingkeyRsFile(
      program,
      paths,
      vKeyJsonPath,
      vKeyRsPath,
      circuitName,
      artifiactPath
    );
  });
});
