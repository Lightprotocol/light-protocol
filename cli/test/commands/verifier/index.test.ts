import test, { expect } from "@oclif/test";
import { initTestEnv, killTestValidator } from "../../../src";
import {
  lightPsp2in2outId,
  lightPsp10in2outId,
  lightPsp4in4outId,
  lightPsp2in2outStorageId,
} from "@lightprotocol/zk.js";

describe("Without preloaded accounts", () => {
  before(async () => {
    await initTestEnv({ skip_system_accounts: true });
  });
  after(async () => {
    await killTestValidator();
  });
  // TODO(vadorovsky): Instead of initializing the Authority manually here,
  // teach `initTestEnv` and `test-validator` to skip certain accounts (in this
  // case - verifier which we want to register later).
  test
    .stdout()
    .command(["merkle-tree-authority:initialize"])
    .it("Initialize Merkle Tree Authority", ({ stdout }) => {
      expect(stdout).to.contain(
        "Merkle Tree Authority initialized successfully"
      );
    });
  test
    .stdout()
    .command(["verifier:register", lightPsp2in2outId.toBase58()])
    .it("Register verifier", ({ stdout }) => {
      expect(stdout).to.contain("Verifier registered successfully");
    });
});

describe("With preloaded accounts", () => {
  before(async () => {
    await initTestEnv({});
  });
  test
    .stdout()
    .command(["verifier:list"])
    .it("List verifiers", ({ stdout }) => {
      expect(stdout).to.contain(lightPsp2in2outId.toBase58());
      expect(stdout).to.contain(lightPsp10in2outId.toBase58());
      expect(stdout).to.contain(lightPsp4in4outId.toBase58());
      expect(stdout).to.contain(lightPsp2in2outStorageId.toBase58());
    });
});
