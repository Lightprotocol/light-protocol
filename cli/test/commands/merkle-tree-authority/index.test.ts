import { expect, test } from "@oclif/test";
import { initTestEnv, killTestValidator } from "../../../src/utils/initTestEnv";

describe("merkle-tree-authority", () => {
  before(async () => {
    await initTestEnv({ skip_system_accounts: true });
  });
  // Other tests require a validator with system accounts. Kill the current one
  // which doesn't have them.
  after(async () => {
    await killTestValidator();
  });
  test
    .stderr()
    .command(["merkle-tree-authority:get"])
    .exit(1)
    .it("Get (uninitialized) Merkle Tree Authority", ({ stderr }) => {
      expect(stderr).to.contain("Merkle Tree Authority is not initialized");
    });
  // First call, Merkle Tree Authority should get initialized successfully.
  test
    .stdout()
    .command(["merkle-tree-authority:initialize"])
    .it("Initialize Merkle Tree Authority", ({ stdout }) => {
      expect(stdout).to.contain(
        "Merkle Tree Authority initialized successfully"
      );
    });
  // Second call, Merkle Tree Authority was already initialized.
  test
    .stdout()
    .command(["merkle-tree-authority:initialize"])
    .it("Merkle Tree Authority already initialized", ({ stdout }) => {
      expect(stdout).to.contain("Merkle Tree Authority already initialized");
    });
  test
    .stdout()
    .command(["merkle-tree-authority:get"])
    .it("Get Merkle Tree Authority", ({ stdout }) => {
      expect(stdout).to.contain("1");
    });
});
