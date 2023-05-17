import { expect, test } from "@oclif/test";
import { Keypair } from "@solana/web3.js";
import { User } from "../../../../light-sdk-ts/src";

// TODO: balance tests
// TODO: history tests
// TODO: change user tests

describe("Airdrop", () => {
  test
    .stdout()
    .command([
      "airdrop",
      "--token=SOL",
      `--amount=1000000000000000`,
      "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
    ])
    .it(
      "airdrop 1000000000000000 sol to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx) => {
        console.log(ctx.stdout);
        expect(ctx.stdout).to.contain("Airdrop successful for user");
      }
    );

  test
    .stdout()
    .command([
      "airdrop",
      "--token=USDC",
      `--amount=1000000`,
      "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
    ])
    .it(
      "airdrop 1000000 usdc to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx) => {
        console.log(ctx.stdout);
        expect(ctx.stdout).to.contain("Airdrop successful for user");
      }
    );
});

describe("Shield Sol", () => {
  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=2.7`])
    .it("Should shield 2.7 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=8.4`])
    .it("Should shield 8.4 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["balance"])
    .it("Should have balance of 11.1 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Total Sol Balance: 11100000000");
    });
});

describe("Unshield Sol", () => {
  let recipient = Keypair.generate().publicKey;

  test
    .stdout()
    .command([
      "unshield",
      "--token=SOL",
      "--amountSol=1.1",
      `--recipientSol=${recipient.toString()}`,
      `--recipientSpl=${recipient.toString()}`,
    ])
    .it("Unshield 1.1 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });

  test
    .stdout()
    .command([
      "unshield",
      "--token=SOL",
      "--amountSol=0.2",
      `--recipientSol=${recipient.toString()}`,
      `--recipientSpl=${recipient.toString()}`,
    ])
    .it("Unshield 0.2 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });
});

describe("Transfer Sol", () => {
  let recipient =
    "JsVivKKxef5rNPdxKc9xsp2WRpomhg1DtEmzm3M8zXCF4b4MBuzy3KQmybErBNrv9SMreTadzpNLQECU4WhsJTw";
  test
    .stdout()
    .command(["transfer", "--token=SOL", "--amountSol=1.5", recipient])
    .it("Transfer 1.5 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Tokens successfully transferred");
    });
});

describe("Merge Utxos", () => {
  test
    .stdout()
    .command([
      "shield",
      "--token=SOL",
      "--amountSol=4",
      "--recipien=sKRXAPf5cAzA28WcMASpEUTVZjc87HSHqVxrGNEW19TjEduQgfFitiVhCnc4EjMhKXSJ15uhTSCRuDUMDmdHhsAt",
    ])
    .it("SHIELD 4 sol to address", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Tokens successfully transferred");
    });
  test
    .stdout()
    .command([
      "config",
      "--secretKey=4roFuZvNJeh9KwZcRYEedsUsh3E8NTY26RvBgqivBbaYA93aCa1bPe4eRFuX6i7p3GwpyfjNvMYNm1t84PoA5g12",
    ])
    .it("runs user update cmd", (ctx) => {
      expect(ctx.stdout).to.contain(
        "Configuration values updated successfully"
      );
    });

  //TODO: find a way to get the commitment from the inbox balance command and then merge it
  // test
  //   .stdout()
  //   .command([
  //     "utxo",
  //     "--token=SOL",
  //     "17863225529163624094949960837831973135282069025808310217647542102196003863196",
  //   ])
  //   .it("should merge the utxo", (ctx) => {
  //     expect(ctx.stdout).to.contain("UTXOs merged successfully!");
  //   });
});

// add the usdc shield test
// add the udc unshield test
// add the usdc transfer test
// histiory tests
// fix usdc airdrop error in first iteration
