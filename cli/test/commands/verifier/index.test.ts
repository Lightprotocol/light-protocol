import test, { expect } from "@oclif/test";
import { initTestEnv, killTestValidator } from "../../../src/utils/initTestEnv";
import {
  verifierProgramOneProgramId,
  verifierProgramStorageProgramId,
  verifierProgramTwoProgramId,
  verifierProgramZeroProgramId,
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
    .command(["verifier:register", verifierProgramZeroProgramId.toBase58()])
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
      expect(stdout).to.contain(verifierProgramZeroProgramId.toBase58());
      expect(stdout).to.contain(verifierProgramOneProgramId.toBase58());
      expect(stdout).to.contain(verifierProgramTwoProgramId.toBase58());
      expect(stdout).to.contain(verifierProgramStorageProgramId.toBase58());
    });
});
