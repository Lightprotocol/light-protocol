import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";

import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  VerifierZero,
  TransactionErrorCode,
  Action,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Relayer,
  FIELD_SIZE,
  merkleTreeProgramId,
  VerifierTwo,
  VerifierOne,
} from "../src";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const verifiers = [new VerifierZero(), new VerifierOne(), new VerifierTwo()];

describe("Transaction Parameters Functional", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer, keypair;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock(mockPubkey3);
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
  });

  it("Transfer Functional", async () => {
    var outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.relayerFee),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
    });

    for (var j in verifiers) {
      const inputUtxos = [deposit_utxo1];
      const outputUtxos = [outputUtxo];

      const params = new TransactionParameters({
        inputUtxos,
        outputUtxos,
        merkleTreePubkey: mockPubkey2,
        verifier: verifiers[j],
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.TRANSFER,
        relayer,
      });

      assert.equal(params.action.toString(), Action.TRANSFER.toString());
      assert.equal(params.publicAmountSpl.toString(), "0");
      assert.equal(
        params.publicAmountSol
          .sub(FIELD_SIZE)
          .mul(new anchor.BN(-1))
          .toString(),
        relayer.relayerFee.toString(),
      );
      assert.equal(
        params.assetPubkeys[0].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(params.assetPubkeys[1].toBase58(), MINT.toBase58());
      assert.equal(
        params.assetPubkeys[2].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(
        params.accounts.recipient?.toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(
        params.accounts.recipientFee?.toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(
        params.accounts.merkleTree.toBase58(),
        mockPubkey2.toBase58(),
      );
      assert.equal(params.accounts.verifierState, undefined);
      assert.equal(params.accounts.programMerkleTree, merkleTreeProgramId);
      assert.equal(
        params.accounts.signingAddress,
        relayer.accounts.relayerPubkey,
      );
      assert.equal(
        params.accounts.signingAddress,
        params.relayer.accounts.relayerPubkey,
      );
      assert.equal(
        params.accounts.authority.toBase58(),
        Transaction.getSignerAuthorityPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        params.verifier.config.in.toString(),
        verifiers[j].config.in.toString(),
      );
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        relayer.accounts.lookUpTable?.toBase58(),
      );
      assert.equal(params.inputUtxos.length, params.verifier.config.in);
      assert.equal(params.outputUtxos.length, params.verifier.config.out);

      for (var i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(),
          inputUtxos[i].getCommitment(),
        );
      }

      for (var i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(),
          outputUtxos[i].getCommitment(),
        );
      }
    }
  });
  it("Deposit Functional", async () => {
    for (var j in verifiers) {
      const outputUtxos = [deposit_utxo1];

      const params = new TransactionParameters({
        outputUtxos,
        merkleTreePubkey: mockPubkey2,
        sender: mockPubkey,
        senderFee: mockPubkey1,
        verifier: verifiers[j],
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.DEPOSIT,
      });

      assert.equal(params.publicAmountSpl.toString(), depositAmount.toString());
      assert.equal(
        params.publicAmountSol.toString(),
        depositFeeAmount.toString(),
      );
      assert.equal(
        params.assetPubkeys[0].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(params.assetPubkeys[1].toBase58(), MINT.toBase58());
      assert.equal(
        params.assetPubkeys[2].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(params.accounts.sender?.toBase58(), mockPubkey.toBase58());
      assert.equal(
        params.accounts.senderFee?.toBase58(),
        TransactionParameters.getEscrowPda(
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(
        params.accounts.merkleTree.toBase58(),
        mockPubkey2.toBase58(),
      );
      assert.equal(params.accounts.verifierState, undefined);
      assert.equal(params.accounts.programMerkleTree, merkleTreeProgramId);
      assert.equal(params.accounts.signingAddress, mockPubkey1);
      assert.equal(
        params.accounts.signingAddress,
        params.relayer.accounts.relayerPubkey,
      );
      assert.equal(
        params.accounts.authority.toBase58(),
        Transaction.getSignerAuthorityPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        params.verifier.config.in.toString(),
        verifiers[j].config.in.toString(),
      );
      assert.equal(params.action.toString(), Action.DEPOSIT.toString());
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        lightProvider.lookUpTable?.toBase58(),
      );
      assert.equal(params.inputUtxos.length, params.verifier.config.in);
      assert.equal(params.outputUtxos.length, params.verifier.config.out);

      for (var i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(),
          outputUtxos[i].getCommitment(),
        );
      }
    }
  });

  it("Withdrawal Functional", async () => {
    for (var j in verifiers) {
      const inputUtxos = [deposit_utxo1];

      const params = new TransactionParameters({
        inputUtxos,
        merkleTreePubkey: mockPubkey2,
        recipient: mockPubkey,
        recipientFee: mockPubkey1,
        verifier: verifiers[j],
        poseidon,
        action: Action.WITHDRAWAL,
        relayer,
      });
      assert.equal(params.action.toString(), Action.WITHDRAWAL.toString());
      assert.equal(
        params.publicAmountSpl
          .sub(FIELD_SIZE)
          .mul(new anchor.BN(-1))
          .toString(),
        depositAmount.toString(),
      );
      assert.equal(
        params.publicAmountSol
          .sub(FIELD_SIZE)
          .mul(new anchor.BN(-1))
          .toString(),
        depositFeeAmount.toString(),
      );
      assert.equal(
        params.assetPubkeys[0].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(params.assetPubkeys[1].toBase58(), MINT.toBase58());
      assert.equal(
        params.assetPubkeys[2].toBase58(),
        SystemProgram.programId.toBase58(),
      );
      assert.equal(
        params.accounts.recipient?.toBase58(),
        mockPubkey.toBase58(),
      );
      assert.equal(
        params.accounts.recipientFee?.toBase58(),
        mockPubkey1.toBase58(),
      );
      assert.equal(
        params.accounts.merkleTree.toBase58(),
        mockPubkey2.toBase58(),
      );
      assert.equal(params.accounts.verifierState, undefined);
      assert.equal(params.accounts.programMerkleTree, merkleTreeProgramId);
      assert.equal(
        params.accounts.signingAddress,
        relayer.accounts.relayerPubkey,
      );
      assert.equal(
        params.accounts.signingAddress,
        params.relayer.accounts.relayerPubkey,
      );
      assert.equal(
        params.accounts.authority.toBase58(),
        Transaction.getSignerAuthorityPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          verifiers[j].verifierProgram!.programId,
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        params.verifier.config.in.toString(),
        verifiers[j].config.in.toString(),
      );
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        relayer.accounts.lookUpTable?.toBase58(),
      );
      assert.equal(params.inputUtxos.length, params.verifier.config.in);
      assert.equal(params.outputUtxos.length, params.verifier.config.out);

      for (var i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(),
          inputUtxos[i].getCommitment(),
        );
      }
    }
  });
});

describe("Test TransactionParameters Methods", () => {
  it("Test getAssetPubkeys", async () => {
    const poseidon = await buildPoseidonOpt();
    let inputUtxos = [new Utxo({ poseidon }), new Utxo({ poseidon })];
    let outputUtxos = [
      new Utxo({
        poseidon,
        amounts: [new anchor.BN(2), new anchor.BN(4)],
        assets: [SystemProgram.programId, MINT],
      }),
      new Utxo({ poseidon }),
    ];

    let { assetPubkeysCircuit, assetPubkeys } = Transaction.getAssetPubkeys(
      inputUtxos,
      outputUtxos,
    );
    assert.equal(
      assetPubkeys[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(assetPubkeys[1].toBase58(), MINT.toBase58());
    assert.equal(
      assetPubkeys[2].toBase58(),
      SystemProgram.programId.toBase58(),
    );

    assert.equal(
      assetPubkeysCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBuffer()).toString(),
    );
    assert.equal(
      assetPubkeysCircuit[1].toString(),
      hashAndTruncateToCircuit(MINT.toBuffer()).toString(),
    );
    assert.equal(assetPubkeysCircuit[2].toString(), "0");
  });

  it("Test getExtAmount", async () => {
    const poseidon = await buildPoseidonOpt();
    let inputUtxos = [new Utxo({ poseidon }), new Utxo({ poseidon })];
    let outputUtxos = [
      new Utxo({
        poseidon,
        amounts: [new anchor.BN(2), new anchor.BN(4)],
        assets: [SystemProgram.programId, MINT],
      }),
      new Utxo({ poseidon }),
    ];
    let { assetPubkeysCircuit, assetPubkeys } = Transaction.getAssetPubkeys(
      inputUtxos,
      outputUtxos,
    );

    let publicAmount = Transaction.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmount.toString(), "2");
    let publicAmountSpl = Transaction.getExternalAmount(
      1,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );

    assert.equal(publicAmountSpl.toString(), "4");

    outputUtxos[1] = new Utxo({
      poseidon,
      amounts: [new anchor.BN(3), new anchor.BN(5)],
      assets: [SystemProgram.programId, MINT],
    });
    let publicAmountSpl2Outputs = Transaction.getExternalAmount(
      1,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSpl2Outputs.toString(), "9");

    let publicAmountSol2Outputs = Transaction.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSol2Outputs.toString(), "5");
  });
});

describe("Test General TransactionParameters Errors", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider, deposit_utxo1, relayer, keypair;

  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock(mockPubkey3);
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.NO_UTXOS_PROVIDED,
          functionName: "constructor",
        });
    }
  });

  it("NO_POSEIDON_HASHER_PROVIDED", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
          functionName: "constructor",
        });
    }
  });

  it("NO_ACTION_PROVIDED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.NO_ACTION_PROVIDED,
          functionName: "constructor",
        });
    }
  });

  it("NO_VERIFIER_PROVIDED", () => {
    expect(() => {
      // @ts-ignore:
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.DEPOSIT,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionParametersErrorCode.NO_VERIFIER_PROVIDED,
        functionName: "constructor",
      });
  });
});

describe("Test TransactionParameters Transfer Errors", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let mockPubkey = SolanaKeypair.generate().publicKey;
  let keypair;

  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock(mockPubkey);
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });

    outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.relayerFee),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
    });

    const params = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      outputUtxos: [outputUtxo],
      merkleTreePubkey: mockPubkey,
      verifier: new VerifierZero(),
      poseidon,
      action: Action.TRANSFER,
      relayer,
    });
  });

  it("RELAYER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.RELAYER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("PUBLIC_AMOUNT_SPL_NOT_ZERO", () => {
    const localOutputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.relayerFee),
        new anchor.BN(0),
      ],
      account: keypair,
    });
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [localOutputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO,
          functionName: "constructor",
        });
    }
  });

  it("PUBLIC_AMOUNT_SOL_NOT_ZERO", () => {
    const localOutputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN(depositAmount)],
      account: keypair,
    });
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [localOutputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO,
          functionName: "constructor",
        });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          recipient: mockPubkey,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          recipientFee: mockPubkey,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          senderFee: mockPubkey,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          merkleTreePubkey: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.TRANSFER,
          sender: mockPubkey,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });
});

describe("Test TransactionParameters Deposit Errors", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let mockPubkey = SolanaKeypair.generate().publicKey;
  let keypair;

  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock(mockPubkey);
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });

    const params = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      verifier: new VerifierZero(),
      sender: mockPubkey,
      senderFee: mockPubkey,
      lookUpTable: mockPubkey,
      poseidon,
      action: Action.DEPOSIT,
    });
  });

  it("SOL_SENDER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SOL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SPL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("LOOK_UP_TABLE_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.LOOK_UP_TABLE_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("RELAYER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.RELAYER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL PUBLIC_AMOUNT_NOT_U64", () => {
    let utxo_sol_amount_no_u641 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN("18446744073709551615"),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
    });
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
    });
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SPL PUBLIC_AMOUNT_NOT_U64", () => {
    let utxo_spl_amount_no_u641 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("18446744073709551615")],
      account: keypair,
    });

    let utxo_spl_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("1")],
      account: keypair,
    });

    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          recipient: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_SENDER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SOL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SPL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("No sender spl needed without spl amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
    });
    for (var verifier in verifiers) {
      // sender fee always needs to be defined because we use it as the signer
      // should work since no spl amount
      new TransactionParameters({
        outputUtxos: [utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        verifier: verifiers[verifier],
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.DEPOSIT,
      });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          recipient: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          senderFee: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.DEPOSIT,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });
});

describe("Test TransactionParameters Withdrawal Errors", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let mockPubkey = SolanaKeypair.generate().publicKey;
  let keypair;

  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer;

  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock(mockPubkey);
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });

    outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.relayerFee),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
    });

    const params = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      verifier: new VerifierZero(),
      poseidon,
      recipient: mockPubkey,
      recipientFee: mockPubkey,
      action: Action.WITHDRAWAL,
      relayer,
    });
  });

  it("SOL_RECIPIENT_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          recipient: mockPubkey,
          // senderFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("RELAYER_UNDEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.WITHDRAWAL,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.RELAYER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL PUBLIC_AMOUNT_NOT_U64", () => {
    let utxo_sol_amount_no_u641 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN("18446744073709551615"),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
    });
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
    });

    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          merkleTreePubkey: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SPL PUBLIC_AMOUNT_NOT_U64", () => {
    let utxo_spl_amount_no_u641 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("18446744073709551615")],
      account: keypair,
    });

    let utxo_spl_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("1")],
      account: keypair,
    });
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          merkleTreePubkey: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          senderFee: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("no recipient spl should work since no spl amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
    });

    for (var verifier in verifiers) {
      // should work since no spl amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipientFee: mockPubkey,
        verifier: verifiers[verifier],
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.WITHDRAWAL,
        relayer,
      });
    }
  });

  it("no recipient sol should work since no sol amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("18446744073709551615")],
      account: keypair,
    });

    for (var verifier in verifiers) {
      // should work since no sol amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        verifier: verifiers[verifier],
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.WITHDRAWAL,
        relayer,
      });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          senderFee: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_DEFINED", () => {
    for (var verifier in verifiers) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          merkleTreePubkey: mockPubkey,
          sender: mockPubkey,
          recipient: mockPubkey,
          recipientFee: mockPubkey,
          verifier: verifiers[verifier],
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.WITHDRAWAL,
          relayer,
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });
});
