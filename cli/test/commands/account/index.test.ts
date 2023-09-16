import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("account", () => {
  before(async () => {
    await initTestEnvIfNeeded();
  });
  test
    .stdout({ print: true })
    .command(["account"])
    .exit(0)
    .it("runs account cmd", ({ stdout }: { stdout: any }) => {
      expect(stdout).to.contain(
        "DVTtJhghZU1hBEbCci4RDpRP1K1eEHZXyYognZ4BNiCBaM8WenG3o6v8CNcKTRD7fVUsSTtae8hU5To1ogrGQDw"
      );
    });
});
