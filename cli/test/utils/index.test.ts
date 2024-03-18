import * as fs from "fs";
import * as os from "os";
import path from "path";
import { assert, expect } from "chai";
import "mocha";

import { getSolanaArgs } from "../../src/utils";

describe("Init test env utils", () => {
  let tempdir: string;

  before(async () => {
    // Ensure that programs (as .so files) are going to be downloaded to a
    // temporary directory.
    tempdir = path.join(os.tmpdir(), "light-protocol-programs");
    fs.mkdirSync(tempdir, { recursive: true });
    process.env.LIGHT_PROTOCOL_PROGRAMS_DIR = tempdir;
  });

  after(async () => {
    fs.rmSync(tempdir, { force: true, recursive: true });
  });

  it("Download program files, prepare arguments", async () => {
    const solanaArgs = await getSolanaArgs({ downloadBinaries: false });

    const splNoopPath = path.join(tempdir, "spl_noop.so");
    expect(solanaArgs).to.contain(splNoopPath);
    assert.isTrue(fs.existsSync(splNoopPath));

    // TODO: add programs after they are ready
  });
});
