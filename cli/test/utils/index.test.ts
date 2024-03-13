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
    const solanaArgs = await getSolanaArgs({});

    const splNoopPath = path.join(tempdir, "spl_noop.so");
    expect(solanaArgs).to.contain(splNoopPath);
    assert.isTrue(fs.existsSync(splNoopPath));

    const merkleTreeProgramPath = path.join(
      tempdir,
      "light_merkle_tree_program.so",
    );
    expect(solanaArgs).to.contain(merkleTreeProgramPath);
    assert.isTrue(fs.existsSync(merkleTreeProgramPath));

    const lightPsp2in2outPath = path.join(tempdir, "light_psp2in2out.so");
    expect(solanaArgs).to.contain(lightPsp2in2outPath);
    assert.isTrue(fs.existsSync(lightPsp2in2outPath));

    const lightPsp2in2outStoragePath = path.join(
      tempdir,
      "light_psp2in2out_storage.so",
    );
    expect(solanaArgs).to.contain(lightPsp2in2outStoragePath);
    assert.isTrue(fs.existsSync(lightPsp2in2outStoragePath));

    const lightPsp4in4outAppStoragePath = path.join(
      tempdir,
      "light_psp4in4out_app_storage.so",
    );
    expect(solanaArgs).to.contain(lightPsp4in4outAppStoragePath);
    assert.isTrue(fs.existsSync(lightPsp4in4outAppStoragePath));

    const lightPsp10in2outPath = path.join(tempdir, "light_psp10in2out.so");
    expect(solanaArgs).to.contain(lightPsp10in2outPath);
    assert.isTrue(fs.existsSync(lightPsp10in2outPath));

    const lightUserRegistryPath = path.join(tempdir, "light_user_registry.so");
    assert.isTrue(fs.existsSync(lightUserRegistryPath));
  });
});
