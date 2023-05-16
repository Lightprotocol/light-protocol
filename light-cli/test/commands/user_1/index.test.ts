import { expect, test } from "@oclif/test";
import { Keypair } from "@solana/web3.js";


// TODO: balance tests
// TODO: history tests
// TODO: change user tests

describe("Shield Sol", () => {
  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=1`])
    .it("runs shield sol 1 cmd", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=3`])
    .it("runs shield sol 3 cmd", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=6`])
    .it("runs shield sol 6 cmd", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });
});

describe("Unshield Sol", () => {
  let recipient = Keypair.generate().publicKey;

  test
    .stdout()
    .command([
      "unshield",
      "--token=SOL",
      "--amountSol=1",
      `--recipientSol=${recipient.toString()}`,
      `--recipientSpl=${recipient.toString()}`,
    ])
    .it("runs shield sol 1 cmd", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });
});

describe("Transfer Sol", () => {
  let recipient =
    "7zf5dv4sc7m2xskswD6J3CtoUoDApyGXRYodUtxTyPAXHmV121zqZR3aqBiL8SHPB4kxSFx12E9aiwmgtGWCjAT";
  test
    .stdout()
    .command([
      "transfer",
      "--token=SOL",
      "--amountSol=2",
      `--recipient=${recipient}`,
    ])
    .it("runs shield sol 1 cmd", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });
});
