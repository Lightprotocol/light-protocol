import { assert, expect } from "chai";
var chaiAsPromised = require("chai-as-promised");
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildBabyjub, buildEddsa } from "circomlibjs";
import { Scalar } from "ffjavascript";

import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  ADMIN_AUTH_KEYPAIR,
  FEE_ASSET,
  functionalCircuitTest,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MERKLE_TREE_KEY,
  MINT,
  Transaction,
  UtxoError,
  UtxoErrorCode,
  TransactionParameters,
  VerifierZero,
  TransactionError,
  TransactionErrorCode,
  ProviderErrorCode,
  Provider,
  Action,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Relayer,
  FIELD_SIZE,
  verifierProgramZeroProgramId,
  MerkleTreeConfig,
  merkleTreeProgramId,
  AUTHORITY,
  VerifierTwo,
  VerifierOne,
} from "../src";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { realpath } from "fs";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

describe("verifier_program", () => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  it("Test poseidon", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let x = new Array(32).fill(1);
    let y = new Array(32).fill(2);

    let hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );
    console.log(new anchor.BN(hash).toArray("le", 32));

    x = new Array(32).fill(3);
    y = new Array(32).fill(3);

    hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );
    console.log(new anchor.BN(hash).toArray("be", 32));
  });

  it("Test Keypair Poseidon Eddsa", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let eddsa = await buildEddsa();
    const babyJub = await buildBabyjub();
    const F = babyJub.F;
    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Account({ poseidon, seed: seed32, eddsa });

    const prvKey = blake2b
      .create(b2params)
      .update(seed32 + "poseidonEddsa")
      .digest();
    const pubKey = eddsa.prv2pub(prvKey);
    k0.getEddsaPublicKey();
    if (k0.poseidonEddsa && k0.poseidonEddsa.publicKey) {
      assert.equal(prvKey.toString(), k0.poseidonEddsa.privateKey.toString());
      assert.equal(
        pubKey[0].toString(),
        k0.poseidonEddsa.publicKey[0].toString(),
      );
      assert.equal(
        pubKey[1].toString(),
        k0.poseidonEddsa.publicKey[1].toString(),
      );
    } else {
      throw new Error("k0.poseidonEddsa undefined");
    }

    const msg = "12321";
    const sigK0 = await k0.signEddsa(msg);
    assert.equal(
      sigK0.toString(),
      eddsa.packSignature(eddsa.signPoseidon(prvKey, F.e(Scalar.e(msg)))),
    );
    assert(eddsa.verifyPoseidon(msg, eddsa.unpackSignature(sigK0), pubKey));
  });

  // TODO: rename to 'Test Account'
  it("Test Keypair", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let seed = "123";
    let seedHash = blake2b.create(b2params).update(seed).digest();
    let encSeed = seed + "encryption";
    let encHash = blake2b.create(b2params).update(encSeed).digest();
    let privkeySeed = seed + "privkey";
    let privkeyHash = blake2b.create(b2params).update(privkeySeed).digest();

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
    try {
      expect(new Account({ poseidon, seed: "123" })).to.throw();
    } catch (e) {
      assert.isTrue(
        e.toString().includes("seed too short length less than 32"),
      );
    }

    const compareKeypairsEqual = (
      k0: Account,
      k1: Account,
      fromPrivkey: Boolean = false,
    ) => {
      assert.equal(k0.privkey.toString(), k1.privkey.toString());
      assert.equal(k0.pubkey.toString(), k1.pubkey.toString());
      assert.equal(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      if (!fromPrivkey) {
        assert.equal(
          k0.encryptionKeypair.publicKey.toString(),
          k1.encryptionKeypair.publicKey.toString(),
        );
      }
    };

    const compareKeypairsNotEqual = (
      k0: Account,
      k1: Account,
      burner = false,
    ) => {
      assert.notEqual(k0.privkey.toString(), k1.privkey.toString());
      assert.notEqual(
        k0.encryptionKeypair.publicKey.toString(),
        k1.encryptionKeypair.publicKey.toString(),
      );
      assert.notEqual(k0.pubkey.toString(), k1.pubkey.toString());
      if (burner) {
        assert.notEqual(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      }
    };

    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Account({ poseidon, seed: seed32 });
    let k00 = new Account({ poseidon, seed: seed32 });
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00);

    // functional reference
    assert.equal(
      k0.encryptionKeypair.publicKey.toString(),
      "79,88,143,40,214,78,70,137,196,5,122,152,24,73,163,196,183,217,173,186,135,188,91,113,160,128,183,111,110,245,183,96",
    );
    assert.equal(
      k0.privkey.toString(),
      "72081772318062199533713901017818635304770734661701934546410527310990294418314",
    );
    assert.equal(
      k0.pubkey.toString(),
      "17617449169454204288593541557256537870126094878332671558512052528902373564643",
    );

    let seedDiff32 = new Uint8Array(32).fill(2).toString();
    let k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    // functional reference burner
    let kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    assert.equal(
      kBurner.encryptionKeypair.publicKey.toString(),
      "118,44,67,51,130,2,17,15,16,119,197,218,27,218,191,249,95,51,193,62,252,27,59,71,151,12,244,206,103,244,155,13",
    );
    assert.equal(
      kBurner.privkey.toString(),
      "81841610170886826015335465607758273107896278528010278185780510216694719969226",
    );
    assert.equal(
      kBurner.pubkey.toString(),
      "3672531747475455051184163226139092471034744667609536681047180780320195966514",
    );
    assert.equal(
      Array.from(kBurner.burnerSeed).toString(),
      "142,254,65,39,85,90,174,142,146,117,207,76,115,140,59,91,85,155,236,166,1,144,219,206,240,188,218,10,215,93,41,213",
    );

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);
    let kBurner0 = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    let kBurner1 = Account.createBurner(poseidon, seed32, new anchor.BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);

    let kBurner2 = Account.fromBurnerSeed(poseidon, kBurner.burnerSeed);
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);

    // fromPrivkey
    let k0Privkey = Account.fromPrivkey(
      poseidon,
      k0.privkey.toBuffer("be", 32),
    );
    compareKeypairsEqual(k0Privkey, k0, true);

    // fromPubkey
    let k0Pubkey = Account.fromPubkey(
      k0.pubkey.toBuffer("be", 32),
      k0.encryptionKeypair.publicKey,
    );
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });

  it("Test Utxo errors", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    let inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
    };

    expect(() => {
      new Utxo({
        poseidon,
        assets: [inputs.assets[1]],
        amounts: inputs.amounts,
        account: inputs.keypair,
        blinding: inputs.blinding,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
        codeMessage: "Length missmatch assets: 1 != amounts: 2",
      });

    expect(() => {
      new Utxo({
        poseidon,
        assets: [MINT, MINT, MINT],
        amounts: [new anchor.BN(1), new anchor.BN(1), new anchor.BN(1)],
        account: inputs.keypair,
        blinding: inputs.blinding,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.EXCEEDED_MAX_ASSETS,
        codeMessage: "assets.length 3 > N_ASSETS 2",
      });

    expect(() => {
      new Utxo({
        poseidon,
        assets: inputs.assets,
        amounts: [inputs.amounts[0], new anchor.BN(-1)],
        account: inputs.keypair,
        blinding: inputs.blinding,
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.NEGATIVE_AMOUNT,
        codeMessage: "amount cannot be negative, amounts[1] = -1",
      });

    expect(() => {
      new Utxo({
        poseidon,
        assets: inputs.assets,
        amounts: inputs.amounts,
        account: inputs.keypair,
        blinding: inputs.blinding,
        appData: new Array(32).fill(1),
      });
    })
      .to.throw(UtxoError)
      .to.include({
        code: UtxoErrorCode.APP_DATA_FROM_BYTES_FUNCTION_UNDEFINED,
        codeMessage: "No appDataFromBytesFn provided",
      });
  });

  it("Test Utxo encryption", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    let inputs = {
      keypair: new Account({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      account: inputs.keypair,
      blinding: inputs.blinding,
    });
    // functional
    assert.equal(utxo0.amounts[0].toString(), amountFee);
    assert.equal(utxo0.amounts[1].toString(), amountToken);
    assert.equal(
      utxo0.assets[0].toBase58(),
      SystemProgram.programId.toBase58(),
    );
    assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(
      utxo0.assetsCircuit[0].toString(),
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    );
    assert.equal(
      utxo0.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString(),
    );
    assert.equal(utxo0.instructionType.toString(), "0");
    assert.equal(utxo0.poolType.toString(), "0");
    assert.equal(
      utxo0.verifierAddress.toString(),
      "11111111111111111111111111111111",
    );
    assert.equal(utxo0.verifierAddressCircuit.toString(), "0");
    assert.equal(
      utxo0.getCommitment()?.toString(),
      "652669139698397343583748072204170820200438709928429876748650598683161543212",
    );
    assert.equal(
      utxo0.getNullifier()?.toString(),
      "17480811615340544191325914403781453306357111339028048073066510246169472538152",
    );

    // toBytes
    const bytes = utxo0.toBytes();
    // fromBytes
    const utxo1 = Utxo.fromBytes({ poseidon, account: inputs.keypair, bytes });
    Utxo.equal(utxo0, utxo1);
    // encrypt
    const encBytes = utxo1.encrypt();

    // decrypt
    const utxo3 = Utxo.decrypt({ poseidon, encBytes, account: inputs.keypair });
    if (utxo3) {
      Utxo.equal(utxo0, utxo3);
    } else {
      throw "decrypt failed";
    }

    // try basic tests for rnd empty utxo
    const utxo4 = new Utxo({ poseidon });
    // toBytes
    const bytes4 = utxo4.toBytes();
    // fromBytes
    const utxo40 = Utxo.fromBytes({
      poseidon,
      account: utxo4.account,
      bytes: bytes4,
    });
    Utxo.equal(utxo4, utxo40);
    // encrypt
    const encBytes4 = utxo4.encrypt();
    const utxo41 = Utxo.decrypt({
      poseidon,
      encBytes: encBytes4,
      account: utxo4.account,
    });
    if (utxo41) {
      Utxo.equal(utxo4, utxo41);
    } else {
      throw "decrypt failed";
    }

    // getNullifier when no privkey
  });

  // test functional circuit
  it("Test functional circuit", async () => {
    await functionalCircuitTest();
  });

  it("Test TransactionParameter errors", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey);

    // let tx = new Transaction({
    //   provider: lightProvider,
    // });

    /**
     * General Transaction Parameter tests
     */
    expect(() => {
      new TransactionParameters({
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
        lookUpTable: lightProvider.lookUpTable,
        action: Action.DEPOSIT,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        functionName: "constructor",
      });

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionParametersErrorCode.NO_ACTION_PROVIDED,
        functionName: "constructor",
      });

    expect(() => {
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

  it("Test Transaction Parameters Deposit errors", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey);
    const relayer = new Relayer(mockPubkey, mockPubkey);
    // let tx = new Transaction({
    //   provider: lightProvider,
    // });

    /**
     * Deposit Transaction Parameter tests
     */
    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        // senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
        // lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.DEPOSIT,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionParametersErrorCode.LOOK_UP_TABLE_UNDEFINED,
        functionName: "constructor",
      });

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    // should work since no sol amount
    // sender fee always needs to be defined because we use it as the signer
    // should work since no spl amount
    new TransactionParameters({
      outputUtxos: [utxo_sol_amount_no_u642],
      merkleTreePubkey: mockPubkey,
      senderFee: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.DEPOSIT,
    });

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        senderFee: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    /*
    let txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      sender: mockPubkey,
      senderFee: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.DEPOSIT
    });
    try {
      
    } catch (error) {
      console.log(err);
      
    }

    expect( () => {
      tx.getAssetPubkeys([], [])
    }).to.throw(TransactionError).to.include({
      code: TransactionErrorCode.NO_UTXOS_PROVIDED,
      functionName: "getAssetPubkeys",
    });

    expect( () => {
      tx.getAssetPubkeys([deposit_utxo1], [])
    }).to.throw(TransactionError).to.include({
      code: TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
      functionName: "getAssetPubkeys",
    });
    */
  });

  it("Test Transaction Parameters Withdrawal errors", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey);
    const relayer = new Relayer(mockPubkey, mockPubkey);

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        // senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
        poseidon,
        action: Action.WITHDRAWAL,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionErrorCode.RELAYER_UNDEFINED,
        functionName: "constructor",
      });

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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipientFee: mockPubkey,
        recipient: mockPubkey,
        sender: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.WITHDRAWAL,
        relayer,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        functionName: "constructor",
      });

    // should work since no spl amount
    new TransactionParameters({
      inputUtxos: [utxo_sol_amount_no_u642],
      merkleTreePubkey: mockPubkey,
      recipientFee: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.WITHDRAWAL,
      relayer,
    });

    // should work since no sol amount
    new TransactionParameters({
      inputUtxos: [utxo_spl_amount_no_u642],
      merkleTreePubkey: mockPubkey,
      recipient: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.WITHDRAWAL,
      relayer,
    });

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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
  });

  const verifiers = [new VerifierZero(), new VerifierOne(), new VerifierTwo()];

  it.only("Test Transaction Parameters Deposit Functional", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey1 = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey3);

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

  it.only("Test Transaction Parameters Withdrawal Functional", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey1 = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    const relayer = new Relayer(mockPubkey3, mockPubkey);

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

  it.only("Test Transaction Parameters Transfer Functional", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey1 = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey);

    const relayer = new Relayer(
      mockPubkey,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );

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

  it("Test Transaction Parameters Transaction errors", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new Account({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let mockPubkey = SolanaKeypair.generate().publicKey;

    let lightProvider = await LightProvider.loadMock(mockPubkey);
    const relayer = new Relayer(mockPubkey, mockPubkey);

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        // senderFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
        poseidon,
        action: Action.WITHDRAWAL,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionErrorCode.RELAYER_UNDEFINED,
        functionName: "constructor",
      });

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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipientFee: mockPubkey,
        recipient: mockPubkey,
        sender: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipient: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        action: Action.WITHDRAWAL,
        relayer,
      });
    })
      .to.throw(TransactioParametersError)
      .to.include({
        code: TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        functionName: "constructor",
      });

    // should work since no spl amount
    new TransactionParameters({
      inputUtxos: [utxo_sol_amount_no_u642],
      merkleTreePubkey: mockPubkey,
      recipientFee: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.WITHDRAWAL,
      relayer,
    });

    // should work since no sol amount
    new TransactionParameters({
      inputUtxos: [utxo_spl_amount_no_u642],
      merkleTreePubkey: mockPubkey,
      recipient: mockPubkey,
      verifier: new VerifierZero(),
      lookUpTable: lightProvider.lookUpTable,
      poseidon,
      action: Action.WITHDRAWAL,
      relayer,
    });

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        senderFee: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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

    expect(() => {
      new TransactionParameters({
        inputUtxos: [deposit_utxo1],
        merkleTreePubkey: mockPubkey,
        sender: mockPubkey,
        recipient: mockPubkey,
        recipientFee: mockPubkey,
        verifier: new VerifierZero(),
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
  });

  it("Test Transaction constructor", async () => {
    let mockPubkey = SolanaKeypair.generate().publicKey;
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let lightProvider: Provider = {};

    expect(() => {
      new Transaction({
        provider: lightProvider,
      });
    })
      .to.throw(TransactionError)
      .to.include({
        code: TransactionErrorCode.POSEIDON_HASHER_UNDEFINED,
        functionName: "constructor",
      });
    lightProvider = { poseidon: poseidon };

    expect(() => {
      new Transaction({
        provider: lightProvider,
      });
    })
      .to.throw(TransactionError)
      .to.include({
        code: ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        functionName: "constructor",
      });

    lightProvider = { poseidon: poseidon, solMerkleTree: 1 };

    expect(() => {
      new Transaction({
        provider: lightProvider,
      });
    })
      .to.throw(TransactionError)
      .to.include({
        code: TransactionErrorCode.WALLET_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("getIndices", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let mockPubkey = SolanaKeypair.generate().publicKey;
    let lightProvider = await LightProvider.loadMock(mockPubkey);
    let tx = new Transaction({
      provider: lightProvider,
    });

    var deposit_utxo1 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(1), new anchor.BN(2)],
    });

    tx.assetPubkeysCircuit = [
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
      hashAndTruncateToCircuit(MINT.toBytes()),
      new anchor.BN(0),
    ];
    const indices1 = tx.getIndices([deposit_utxo1]);
    assert.equal(indices1[0][0][0], "1");
    assert.equal(indices1[0][0][1], "0");
    assert.equal(indices1[0][0][2], "0");
    assert.equal(indices1[0][1][0], "0");
    assert.equal(indices1[0][1][1], "1");
    assert.equal(indices1[0][1][2], "0");

    const indices2 = tx.getIndices([deposit_utxo1, deposit_utxo1]);
    assert.equal(indices2[0][0][0], "1");
    assert.equal(indices2[0][0][1], "0");
    assert.equal(indices2[0][0][2], "0");
    assert.equal(indices2[0][1][0], "0");
    assert.equal(indices2[0][1][1], "1");
    assert.equal(indices2[0][1][2], "0");

    var deposit_utxo2 = new Utxo({
      poseidon,
      assets: [FEE_ASSET],
      amounts: [new anchor.BN(1)],
    });

    const indices3 = tx.getIndices([deposit_utxo2]);
    assert.equal(indices3[0][0][0], "1");
    assert.equal(indices3[0][0][1], "0");
    assert.equal(indices3[0][0][2], "0");
    assert.equal(indices3[0][1][0], "0");
    assert.equal(indices3[0][1][1], "0");
    assert.equal(indices3[0][1][2], "0");

    var deposit_utxo3 = new Utxo({
      poseidon,
    });

    const indices4 = tx.getIndices([deposit_utxo3]);
    assert.equal(indices4[0][0][0], "0");
    assert.equal(indices4[0][0][1], "0");
    assert.equal(indices4[0][0][2], "0");
    assert.equal(indices4[0][1][0], "0");
    assert.equal(indices4[0][1][1], "0");
    assert.equal(indices4[0][1][2], "0");

    var deposit_utxo4 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN(2)],
    });

    const indices5 = tx.getIndices([deposit_utxo4]);
    assert.equal(indices5[0][0][0], "1");
    assert.equal(indices5[0][0][1], "0");
    assert.equal(indices5[0][0][2], "0");
    assert.equal(indices5[0][1][0], "0");
    assert.equal(indices5[0][1][1], "1");
    assert.equal(indices5[0][1][2], "0");

    const indices6 = tx.getIndices([deposit_utxo3, deposit_utxo4]);
    assert.equal(indices6[0][0][0], "0");
    assert.equal(indices6[0][0][1], "0");
    assert.equal(indices6[0][0][2], "0");
    assert.equal(indices6[0][1][0], "0");
    assert.equal(indices6[0][1][1], "0");
    assert.equal(indices6[0][1][2], "0");

    assert.equal(indices6[1][0][0], "1");
    assert.equal(indices6[1][0][1], "0");
    assert.equal(indices6[1][0][2], "0");
    assert.equal(indices6[1][1][0], "0");
    assert.equal(indices6[1][1][1], "1");
    assert.equal(indices6[1][1][2], "0");

    var deposit_utxo5 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(2), new anchor.BN(0)],
    });

    const indices7 = tx.getIndices([deposit_utxo5]);
    assert.equal(indices7[0][0][0], "1");
    assert.equal(indices7[0][0][1], "0");
    assert.equal(indices7[0][0][2], "0");
    assert.equal(indices7[0][1][0], "0");
    assert.equal(indices7[0][1][1], "1");
    assert.equal(indices7[0][1][2], "0");
  });
});
