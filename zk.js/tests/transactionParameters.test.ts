import { assert, expect } from "chai";

import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import { BN, utils } from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  TransactionErrorCode,
  Action,
  TransactionParametersError,
  TransactionParametersErrorCode,
  Relayer,
  FIELD_SIZE,
  merkleTreeProgramId,
  AUTHORITY,
  Utxo,
  Account,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  MerkleTreeConfig,
  BN_0,
  BN_2,
} from "../src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const VERIFIER_IDLS = [
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
];

describe("Transaction Parameters Functional", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  const mockPubkey = SolanaKeypair.generate().publicKey;
  const mockPubkey1 = SolanaKeypair.generate().publicKey;
  const mockPubkey2 = SolanaKeypair.generate().publicKey;
  const mockPubkey3 = SolanaKeypair.generate().publicKey;
  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    relayer: Relayer,
    account: Account;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    lightProvider = await LightProvider.loadMock();

    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey3, mockPubkey, new BN(5000));
    account = new Account({ hasher: hasher, seed: seed32 });
    shieldUtxo1 = new Utxo({
      hasher: hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  // TODO(vadorovsky): This test fails because of insufficient size of the
  // borsh buffer. Once we are closer to implementing multisig, we need to fix
  // that problem properly.
  it.skip("Serialization Transfer Functional", async () => {
    const outputUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new BN(shieldFeeAmount).sub(relayer.getRelayerFee()),
        new BN(shieldAmount),
      ],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const j = 0;
    const inputUtxos = [shieldUtxo1];
    const outputUtxos = [outputUtxo];

    const paramsOriginal = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      hasher,
      action: Action.TRANSFER,
      relayer,
      verifierIdl: VERIFIER_IDLS[j],
      account,
    });

    const bytes = await paramsOriginal.toBytes();

    const params = await TransactionParameters.fromBytes({
      hasher,
      utxoIdls: [IDL_LIGHT_PSP2IN2OUT],
      relayer,
      bytes,
      verifierIdl: VERIFIER_IDLS[j],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    assert.equal(params.action.toString(), Action.TRANSFER.toString());
    assert.equal(params.publicAmountSpl.toString(), "0");
    assert.equal(
      params.publicAmountSol.sub(FIELD_SIZE).mul(new BN(-1)).toString(),
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
      MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    );
    const verifierState = PublicKey.findProgramAddressSync(
      [
        params.accounts.signingAddress!.toBytes(),
        utils.bytes.utf8.encode("VERIFIER_STATE"),
      ],
      TransactionParameters.getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT),
    )[0];
    assert.equal(params.accounts.verifierState, verifierState);
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
      params.inputUtxos.length,
      TransactionParameters.getVerifierConfig(params.verifierIdl).in,
    );
    assert.equal(
      params.outputUtxos.length,
      TransactionParameters.getVerifierConfig(params.verifierIdl).out,
    );

    for (const i in inputUtxos) {
      assert.equal(
        params.inputUtxos[i].getCommitment(hasher),
        inputUtxos[i].getCommitment(hasher),
      );
    }

    for (const i in outputUtxos) {
      assert.equal(
        params.outputUtxos[i].getCommitment(hasher),
        outputUtxos[i].getCommitment(hasher),
      );
    }
  });

  it("Transfer Functional", async () => {
    const outputUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new BN(shieldFeeAmount).sub(relayer.getRelayerFee()),
        new BN(shieldAmount),
      ],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    for (const j in VERIFIER_IDLS) {
      const inputUtxos = [shieldUtxo1];
      const outputUtxos = [outputUtxo];

      const params = new TransactionParameters({
        inputUtxos,
        outputUtxos,
        eventMerkleTreePubkey: mockPubkey2,
        transactionMerkleTreePubkey: mockPubkey2,
        hasher,
        action: Action.TRANSFER,
        relayer,
        verifierIdl: VERIFIER_IDLS[j],
        account,
      });

      assert.equal(params.action.toString(), Action.TRANSFER.toString());
      assert.equal(params.publicAmountSpl.toString(), "0");
      assert.equal(
        params.publicAmountSol.sub(FIELD_SIZE).mul(new BN(-1)).toString(),
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
      const verifierState = PublicKey.findProgramAddressSync(
        [
          params.accounts.signingAddress!.toBytes(),
          utils.bytes.utf8.encode("VERIFIER_STATE"),
        ],
        TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
      )[0];
      assert.equal(
        params.accounts.verifierState?.toBase58(),
        verifierState.toBase58(),
      );
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
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (const i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(hasher),
          inputUtxos[i].getCommitment(hasher),
        );
      }

      for (const i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(hasher),
          outputUtxos[i].getCommitment(hasher),
        );
      }
    }
  });
  it("Shield Functional", async () => {
    for (const j in VERIFIER_IDLS) {
      const outputUtxos = [shieldUtxo1];

      const params = new TransactionParameters({
        outputUtxos,
        eventMerkleTreePubkey: mockPubkey2,
        transactionMerkleTreePubkey: mockPubkey2,
        senderSpl: mockPubkey,
        senderSol: mockPubkey1,
        hasher,
        action: Action.SHIELD,

        verifierIdl: VERIFIER_IDLS[j],
        account,
      });

      assert.equal(params.publicAmountSpl.toString(), shieldAmount.toString());
      assert.equal(
        params.publicAmountSol.toString(),
        shieldFeeAmount.toString(),
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
      const verifierState = PublicKey.findProgramAddressSync(
        [
          params.accounts.signingAddress!.toBytes(),
          utils.bytes.utf8.encode("VERIFIER_STATE"),
        ],
        TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
      )[0];
      assert.equal(
        params.accounts.verifierState?.toBase58(),
        verifierState.toBase58(),
      );
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
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (const i in outputUtxos) {
        assert.equal(
          params.outputUtxos[i].getCommitment(hasher),
          outputUtxos[i].getCommitment(hasher),
        );
      }
    }
  });

  it("Unshield Functional", async () => {
    for (const j in VERIFIER_IDLS) {
      const inputUtxos = [shieldUtxo1];

      const params = new TransactionParameters({
        inputUtxos,
        eventMerkleTreePubkey: mockPubkey2,
        transactionMerkleTreePubkey: mockPubkey2,
        recipientSpl: mockPubkey,
        recipientSol: mockPubkey1,
        hasher,
        action: Action.UNSHIELD,
        relayer,

        verifierIdl: VERIFIER_IDLS[j],
        account,
      });
      assert.equal(params.action.toString(), Action.UNSHIELD.toString());
      assert.equal(
        params.publicAmountSpl.sub(FIELD_SIZE).mul(new BN(-1)).toString(),
        shieldAmount.toString(),
      );
      assert.equal(
        params.publicAmountSol.sub(FIELD_SIZE).mul(new BN(-1)).toString(),
        shieldFeeAmount.toString(),
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
      const verifierState = PublicKey.findProgramAddressSync(
        [
          params.accounts.signingAddress!.toBytes(),
          utils.bytes.utf8.encode("VERIFIER_STATE"),
        ],
        TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j]),
      )[0];
      assert.equal(
        params.accounts.verifierState?.toBase58(),
        verifierState.toBase58(),
      );
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
        params.inputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).in,
      );
      assert.equal(
        params.outputUtxos.length,
        TransactionParameters.getVerifierConfig(params.verifierIdl).out,
      );

      for (const i in inputUtxos) {
        assert.equal(
          params.inputUtxos[i].getCommitment(hasher),
          inputUtxos[i].getCommitment(hasher),
        );
      }
    }
  });
});

describe("Test TransactionParameters Methods", () => {
  let lightProvider: LightProvider;
  it("Test getAssetPubkeys", async () => {
    lightProvider = await LightProvider.loadMock();
    const hasher = await WasmHasher.getInstance();
    const account = new Account({ hasher });
    const inputUtxos = [
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: account.pubkey,
      }),
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: account.pubkey,
      }),
    ];
    const outputUtxos = [
      new Utxo({
        hasher,
        amounts: [BN_2, new BN(4)],
        assets: [SystemProgram.programId, MINT],
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
    ];

    const { assetPubkeysCircuit, assetPubkeys } =
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
    const hasher = await WasmHasher.getInstance();
    const inputUtxos = [
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
    ];
    const outputUtxos = [
      new Utxo({
        hasher,
        amounts: [BN_2, new BN(4)],
        assets: [SystemProgram.programId, MINT],
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
      new Utxo({
        hasher,
        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        publicKey: new Account({ hasher }).pubkey,
      }),
    ];
    const { assetPubkeysCircuit } = TransactionParameters.getAssetPubkeys(
      inputUtxos,
      outputUtxos,
    );

    const publicAmountSol = TransactionParameters.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSol.toString(), "2");
    const publicAmountSpl = TransactionParameters.getExternalAmount(
      1,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );

    assert.equal(publicAmountSpl.toString(), "4");

    outputUtxos[1] = new Utxo({
      hasher,
      amounts: [new BN(3), new BN(5)],
      assets: [SystemProgram.programId, MINT],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: new Account({ hasher }).pubkey,
    });
    const publicAmountSpl2Outputs = TransactionParameters.getExternalAmount(
      1,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSpl2Outputs.toString(), "9");

    const publicAmountSol2Outputs = TransactionParameters.getExternalAmount(
      0,
      inputUtxos,
      outputUtxos,
      assetPubkeysCircuit,
    );
    assert.equal(publicAmountSol2Outputs.toString(), "5");
  });
});

describe("Test General TransactionParameters Errors", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  const mockPubkey = SolanaKeypair.generate().publicKey;
  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    account: Account;

  before(async () => {
    hasher = await WasmHasher.getInstance();
    // TODO: make fee mandatory
    account = new Account({ hasher, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.NO_UTXOS_PROVIDED,
          functionName: "constructor",
        });
    }
  });

  it("NO_POSEIDON_HASHER_PROVIDED", async () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
          functionName: "constructor",
        });
    }
  });

  it("NO_ACTION_PROVIDED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        // @ts-ignore:
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
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
        outputUtxos: [shieldUtxo1],
        transactionMerkleTreePubkey: mockPubkey,
        senderSpl: mockPubkey,
        senderSol: mockPubkey,
        hasher,
        action: Action.SHIELD,
      });
    })
      .to.throw(TransactionParametersError)
      .to.include({
        code: TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED,
        functionName: "constructor",
      });
  });
});

describe("Test TransactionParameters Transfer Errors", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const mockPubkey = SolanaKeypair.generate().publicKey;
  let account: Account;
  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    outputUtxo: Utxo,
    relayer: Relayer;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey, mockPubkey, new BN(5000));
    account = new Account({ hasher, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    outputUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new BN(shieldFeeAmount).sub(relayer.getRelayerFee()),
        new BN(shieldAmount),
      ],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("RELAYER_UNDEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [outputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.RELAYER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("PUBLIC_AMOUNT_SPL_NOT_ZERO", () => {
    const localOutputUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(relayer.getRelayerFee()), BN_0],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [localOutputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO,
          functionName: "constructor",
        });
    }
  });

  it("PUBLIC_AMOUNT_SOL_NOT_ZERO", () => {
    const localOutputUtxo = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [localOutputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO,
          functionName: "constructor",
        });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [outputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          recipientSpl: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [outputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          recipientSol: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [outputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          senderSol: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          outputUtxos: [outputUtxo],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          hasher,
          action: Action.TRANSFER,
          senderSpl: mockPubkey,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });
});

describe("Test TransactionParameters Deposit Errors", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const mockPubkey = SolanaKeypair.generate().publicKey;
  let account: Account;

  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    relayer: Relayer;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey, mockPubkey, new BN(5000));
    account = new Account({ hasher, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("SOL_SENDER_UNDEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.SOL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_UNDEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.SPL_SENDER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("RELAYER_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.RELAYER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL PUBLIC_AMOUNT_NOT_U64", () => {
    const utxo_sol_amount_no_u641 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const utxo_sol_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), BN_0],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SPL PUBLIC_AMOUNT_NOT_U64", () => {
    const utxo_spl_amount_no_u641 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN("18446744073709551615")],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const utxo_spl_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN("1")],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("No senderSpl spl needed without spl amount", () => {
    const utxo_sol_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), BN_0],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    for (const verifier in VERIFIER_IDLS) {
      // senderSpl fee always needs to be defined because we use it as the signer
      // should work since no spl amount
      new TransactionParameters({
        outputUtxos: [utxo_sol_amount_no_u642],
        eventMerkleTreePubkey: mockPubkey,
        transactionMerkleTreePubkey: mockPubkey,
        senderSol: mockPubkey,
        hasher,
        action: Action.SHIELD,
        verifierIdl: VERIFIER_IDLS[verifier],
        account,
      });
    }
  });

  it("SPL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL_RECIPIENT_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          outputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          senderSol: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.SHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          functionName: "constructor",
        });
    }
  });
});

describe("Test TransactionParameters Withdrawal Errors", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const mockPubkey = SolanaKeypair.generate().publicKey;
  let account: Account;

  let hasher: Hasher,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    relayer: Relayer;

  before(async () => {
    hasher = await WasmHasher.getInstance();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey, mockPubkey, new BN(5000));
    account = new Account({ hasher, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("SOL_RECIPIENT_UNDEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          // senderSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("RELAYER_UNDEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionErrorCode.RELAYER_UNDEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SOL PUBLIC_AMOUNT_NOT_U64", () => {
    const utxo_sol_amount_no_u641 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), new BN(shieldAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const utxo_sol_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), BN_0],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SPL PUBLIC_AMOUNT_NOT_U64", () => {
    const utxo_spl_amount_no_u641 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN("18446744073709551615")],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const utxo_spl_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN("1")],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          functionName: "constructor",
        });
    }
  });

  it("SOL_SENDER_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSol: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("SPL_SENDER_DEFINED", () => {
    for (const verifier in VERIFIER_IDLS) {
      expect(() => {
        new TransactionParameters({
          inputUtxos: [shieldUtxo1],
          eventMerkleTreePubkey: mockPubkey,
          transactionMerkleTreePubkey: mockPubkey,
          senderSpl: mockPubkey,
          recipientSpl: mockPubkey,
          recipientSol: mockPubkey,
          hasher,
          action: Action.UNSHIELD,
          relayer,
          verifierIdl: VERIFIER_IDLS[verifier],
          account,
        });
      })
        .to.throw(TransactionParametersError)
        .to.include({
          code: TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          functionName: "constructor",
        });
    }
  });

  it("no recipientSpl spl should work since no spl amount", () => {
    const utxo_sol_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN("18446744073709551615"), BN_0],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    for (const verifier in VERIFIER_IDLS) {
      // should work since no spl amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        eventMerkleTreePubkey: mockPubkey,
        transactionMerkleTreePubkey: mockPubkey,
        recipientSol: mockPubkey,
        hasher,
        action: Action.UNSHIELD,
        relayer,
        verifierIdl: VERIFIER_IDLS[verifier],
        account,
      });
    }
  });

  it("no recipientSpl sol should work since no sol amount", () => {
    const utxo_sol_amount_no_u642 = new Utxo({
      hasher,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, new BN("18446744073709551615")],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    for (const verifier in VERIFIER_IDLS) {
      // should work since no sol amount
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u642],
        eventMerkleTreePubkey: mockPubkey,
        transactionMerkleTreePubkey: mockPubkey,
        recipientSpl: mockPubkey,
        hasher,
        action: Action.UNSHIELD,
        relayer,
        verifierIdl: VERIFIER_IDLS[verifier],
        account,
      });
    }
  });
});
