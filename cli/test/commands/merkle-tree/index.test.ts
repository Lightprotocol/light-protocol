import { expect, test } from "@oclif/test";
import { initTestEnv } from "../../../src/utils/initTestEnv";

describe("merkle-tree", () => {
  before(async () => {
    await initTestEnv({ skipSystemAccounts: true });
  });
  // TODO(vadorovsky): Teach `initTestEnv` to initialize only some accounts,
  // then we will be able to not initialize the Merkle Tree Authority this
  // way.
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
    .command(["merkle-tree:initialize"])
    .it("Initialize new Merkle Trees", ({ stdout }) => {
      expect(stdout).to.contain("Merkle Trees initialized successfully");
    });
});
