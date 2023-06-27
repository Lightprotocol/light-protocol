import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  TransactionErrorCode,
  Action,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Relayer,
  FIELD_SIZE,
  merkleTreeProgramId,
  AUTHORITY,
  Utxo,
  Account,
  TRANSACTION_MERKLE_TREE_KEY,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_TWO,
} from "../src";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const VERIFIER_IDLS = [
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_TWO,
];

describe("Transaction Parameters Functional", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer, keypair;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    lightProvider = await LightProvider.loadMock();

    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey3,
      lightProvider.lookUpTable,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("Serialization Transfer Functional", async () => {
    var outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let j = 0;
    const inputUtxos = [deposit_utxo1];
    const outputUtxos = [outputUtxo];

    const paramsOriginal = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.TRANSFER,
      relayer,
      verifierIdl: VERIFIER_IDLS[j],
    });

    let bytes = await paramsOriginal.toBytes();

    let params = await TransactionParameters.fromBytes({
      poseidon,
      utxoIdls: [IDL_VERIFIER_PROGRAM_ZERO],
      relayer,
      bytes,
      verifierIdl: VERIFIER_IDLS[j],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    assert.equal(params.action.toString(), Action.TRANSFER.toString());
    assert.equal(params.publicAmountSpl.toString(), "0");
    assert.equal(
      params.publicAmountSol.sub(FIELD_SIZE).mul(new anchor.BN(-1)).toString(),
      relayer.getRelayerFee().toString(),
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
      params.accounts.recipientSpl?.toBase58(),
      AUTHORITY.toBase58(),
    );
    assert.equal(
      params.accounts.recipientSol?.toBase58(),
      AUTHORITY.toBase58(),
    );
    assert.equal(
      params.accounts.transactionMerkleTree.toBase58(),
      TRANSACTION_MERKLE_TREE_KEY.toBase58(),
    );
    assert.equal(params.accounts.verifierState, undefined);
    assert.equal(params.accounts.programMerkleTree, merkleTreeProgramId);
    assert.equal(
      params.accounts.signingAddress?.toBase58(),
      relayer.accounts.relayerPubkey.toBase58(),
    );
    assert.equal(
      params.accounts.signingAddress?.toBase58(),
      params.relayer.accounts.relayerPubkey.toBase58(),
    );
    assert.equal(
      params.accounts.authority.toBase58(),
      Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
      ).toBase58(),
    );
    assert.equal(
      params.accounts.registeredVerifierPda.toBase58(),
      Transaction.getRegisteredVerifierPda(
        merkleTreeProgramId,
        TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
      ).toBase58(),
    );

    assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
    assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
    assert.equal(
      params.accounts.tokenAuthority?.toBase58(),
      Transaction.getTokenAuthority().toBase58(),
    );
    assert.equal(
      TransactionParameters.getVerifierConfig(params.verifierIdl).in.toString(),
      TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString(),
    );
    assert.equal(
      params.relayer.accounts.lookUpTable.toBase58(),
      relayer.accounts.lookUpTable?.toBase58(),
    );
    assert.equal(
      params.inputUtxos.length,
      TransactionParameters.getVerifierConfig(params.verifierIdl).in,
    );
    assert.equal(
      params.outputUtxos.length,
      TransactionParameters.getVerifierConfig(params.verifierIdl).out,
    );

    for (var i in inputUtxos) {
      assert.equal(
        params.inputUtxos[i].getCommitment(poseidon),
        inputUtxos[i].getCommitment(poseidon),
      );
    }

    for (var i in outputUtxos) {
      assert.equal(
        params.outputUtxos[i].getCommitment(poseidon),
        outputUtxos[i].getCommitment(poseidon),
      );
    }
  });

  it("Transfer Functional", async () => {
    var outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    for (var j in VERIFIER_IDLS) {
      const inputUtxos = [deposit_utxo1];
      const outputUtxos = [outputUtxo];

      const params = new TransactionParameters({
        inputUtxos,
        outputUtxos,
        transactionMerkleTreePubkey: mockPubkey2,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.TRANSFER,
        relayer,

        verifierIdl: VERIFIER_IDLS[j],
      });

      assert.equal(params.action.toString(), Action.TRANSFER.toString());
      assert.equal(params.publicAmountSpl.toString(), "0");
      assert.equal(
        params.publicAmountSol
          .sub(FIELD_SIZE)
          .mul(new anchor.BN(-1))
          .toString(),
        relayer.getRelayerFee().toString(),
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
        params.accounts.recipientSpl?.toBase58(),
        AUTHORITY.toBase58(),
      );
      assert.equal(
        params.accounts.recipientSol?.toBase58(),
        AUTHORITY.toBase58(),
      );
      assert.equal(
        params.accounts.transactionMerkleTree.toBase58(),
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
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        TransactionParameters.getVerifierConfig(
          params.verifierIdl,
        ).in.toString(),
        TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString(),
      );
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        relayer.accounts.lookUpTable?.toBase58(),
      );
      assert.equal(
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (var i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(poseidon),
          inputUtxos[i].getCommitment(),
        );
      }

      for (var i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(poseidon),
          outputUtxos[i].getCommitment(poseidon),
        );
      }
    }
  });
  it("Deposit Functional", async () => {
    for (var j in VERIFIER_IDLS) {
      const outputUtxos = [deposit_utxo1];

      const params = new TransactionParameters({
        outputUtxos,
        transactionMerkleTreePubkey: mockPubkey2,
        senderSpl: mockPubkey,
        senderSol: mockPubkey1,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.SHIELD,

        verifierIdl: VERIFIER_IDLS[j],
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
      assert.equal(
        params.accounts.senderSpl?.toBase58(),
        mockPubkey.toBase58(),
      );
      assert.equal(
        params.accounts.senderSol?.toBase58(),
        TransactionParameters.getEscrowPda(
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(
        params.accounts.transactionMerkleTree.toBase58(),
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
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        TransactionParameters.getVerifierConfig(
          params.verifierIdl,
        ).in.toString(),
        TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString(),
      );
      assert.equal(params.action.toString(), Action.SHIELD.toString());
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        lightProvider.lookUpTable?.toBase58(),
      );
      assert.equal(
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (var i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(poseidon),
          outputUtxos[i].getCommitment(),
        );
      }
    }
  });

  it("Withdrawal Functional", async () => {
    for (var j in VERIFIER_IDLS) {
      const inputUtxos = [deposit_utxo1];

      const params = new TransactionParameters({
        inputUtxos,
        transactionMerkleTreePubkey: mockPubkey2,
        recipientSpl: mockPubkey,
        recipientSol: mockPubkey1,
        poseidon,
        action: Action.UNSHIELD,
        relayer,

        verifierIdl: VERIFIER_IDLS[j],
      });
      assert.equal(params.action.toString(), Action.UNSHIELD.toString());
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
        params.accounts.recipientSpl?.toBase58(),
        mockPubkey.toBase58(),
      );
      assert.equal(
        params.accounts.recipientSol?.toBase58(),
        mockPubkey1.toBase58(),
      );
      assert.equal(
        params.accounts.transactionMerkleTree.toBase58(),
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
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(
        params.accounts.registeredVerifierPda.toBase58(),
        Transaction.getRegisteredVerifierPda(
          merkleTreeProgramId,
          TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
        ).toBase58(),
      );
      assert.equal(params.accounts.systemProgramId, SystemProgram.programId);
      assert.equal(params.accounts.tokenProgram, TOKEN_PROGRAM_ID);
      assert.equal(
        params.accounts.tokenAuthority?.toBase58(),
        Transaction.getTokenAuthority().toBase58(),
      );
      assert.equal(
        TransactionParameters.getVerifierConfig(
          params.verifierIdl,
        ).in.toString(),
        TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString(),
      );
      assert.equal(
        params.relayer.accounts.lookUpTable.toBase58(),
        relayer.accounts.lookUpTable?.toBase58(),
      );
      assert.equal(
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (var i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(poseidon),
          inputUtxos[i].getCommitment(),
        );
      }
    }
  });
});

describe("Test TransactionParameters Methods", () => {
  let lightProvider: LightProvider;
  it("Test getAssetPubkeys", async () => {
    lightProvider = await LightProvider.loadMock();
    const poseidon = await buildPoseidonOpt();
    let inputUtxos = [
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
    ];
    let outputUtxos = [
      new Utxo({
        poseidon,
        amounts: [new anchor.BN(2), new anchor.BN(4)],
        assets: [SystemProgram.programId, MINT],
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
    ];

    let { assetPubkeysCircuit, assetPubkeys } =
      TransactionParameters.getAssetPubkeys(inputUtxos, outputUtxos);
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
    let inputUtxos = [
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
    ];
    let outputUtxos = [
      new Utxo({
        poseidon,
        amounts: [new anchor.BN(2), new anchor.BN(4)],
        assets: [SystemProgram.programId, MINT],
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
      new Utxo({
        poseidon,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          lightProvider.lookUpTables.verifierProgramLookupTable,
      }),
    ];
    let { assetPubkeysCircuit, assetPubkeys } =
      TransactionParameters.getAssetPubkeys(inputUtxos, outputUtxos);

    let publicAmountSol = TransactionParameters.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSol.toString(), "2");
    let publicAmountSpl = TransactionParameters.getExternalAmount(
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let publicAmountSpl2Outputs = TransactionParameters.getExternalAmount(
      1,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSpl2Outputs.toString(), "9");

    let publicAmountSol2Outputs = TransactionParameters.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSol2Outputs.toString(), "5");
  });
});

describe("Test General TransactionParameters Errors", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
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
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,

          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          verifierIdl: VERIFIER_IDLS[verifier],
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
        transactionMerkleTreePubkey: mockPubkey,
        senderSpl: mockPubkey,
        senderSol: mockPubkey,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.SHIELD,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED,
        functionName: "constructor",
      });
  });
});

describe("Test TransactionParameters Transfer Errors", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
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
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const params = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      outputUtxos: [outputUtxo],
      transactionMerkleTreePubkey: mockPubkey,
      poseidon,
      action: Action.TRANSFER,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("RELAYER_UNDEFINED", () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,

          verifierIdl: VERIFIER_IDLS[verifier],
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
        new anchor.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
        new anchor.BN(0),
      ],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [localOutputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [localOutputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          recipientSpl: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          recipientSol: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          senderSol: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          outputUtxos: [outputUtxo],
          transactionMerkleTreePubkey: mockPubkey,
          poseidon,
          action: Action.TRANSFER,
          senderSpl: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
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
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const params = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: mockPubkey,
      lookUpTable: mockPubkey,
      poseidon,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("SOL_SENDER_UNDEFINED", () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let utxo_spl_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("1")],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionErrorCode.SPL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("No senderSpl spl needed without spl amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    for (var verifier in VERIFIER_IDLS) {
      // senderSpl fee always needs to be defined because we use it as the signer
      // should work since no spl amount
      new TransactionParameters({
        outputUtxos: [utxo_sol_amount_no_u642],
        transactionMerkleTreePubkey: mockPubkey,
        senderSol: mockPubkey,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.SHIELD,
        verifierIdl: VERIFIER_IDLS[verifier],
      });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
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
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    outputUtxo = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new anchor.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
        new anchor.BN(depositAmount),
      ],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("SOL_RECIPIENT_UNDEFINED", () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          // senderSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          poseidon,
          action: Action.UNSHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let utxo_spl_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("1")],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
        });
      })
        .to.throw(TransactioParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("no recipientSpl spl should work since no spl amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN("18446744073709551615"), new anchor.BN(0)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    for (var verifier in VERIFIER_IDLS) {
      // should work since no spl amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        transactionMerkleTreePubkey: mockPubkey,
        recipientSol: mockPubkey,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.UNSHIELD,
        relayer,
        verifierIdl: VERIFIER_IDLS[verifier],
      });
    }
  });

  it("no recipientSpl sol should work since no sol amount", () => {
    let utxo_sol_amount_no_u642 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN("18446744073709551615")],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    for (var verifier in VERIFIER_IDLS) {
      // should work since no sol amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        transactionMerkleTreePubkey: mockPubkey,
        recipientSpl: mockPubkey,
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.UNSHIELD,
        relayer,
        verifierIdl: VERIFIER_IDLS[verifier],
      });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
    for (var verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [deposit_utxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          lookUpTable: lightProvider.lookUpTable,
          poseidon,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
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
