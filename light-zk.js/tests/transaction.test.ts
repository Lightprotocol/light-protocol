import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";

import { it } from "mocha";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  TransactionErrorCode,
  Action,
  Relayer,
  AUTHORITY,
  TransactionError,
  ProviderErrorCode,
  SolMerkleTreeErrorCode,
  Utxo,
  Account,
  MerkleTree,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_TWO,
  IDL_VERIFIER_PROGRAM_STORAGE,
  MESSAGE_MERKLE_TREE_KEY,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Transaction Error Tests", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon: any,
    lightProvider: LightProvider,
    deposit_utxo1: Utxo,
    relayer,
    keypair,
    params: TransactionParameters;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey3, mockPubkey, new anchor.BN(5000));
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
    params = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet?.publicKey,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("Constructor PROVIDER_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore:
      new Transaction({
        params,
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.PROVIDER_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("Constructor POSEIDON_HASHER_UNDEFINED", async () => {
    expect(() => {
      new Transaction({
        // @ts-ignore:
        provider: {},
        params,
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.POSEIDON_HASHER_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("Constructor SOL_MERKLE_TREE_UNDEFINED", async () => {
    expect(() => {
      new Transaction({
        // @ts-ignore:
        provider: { poseidon },
        params,
      });
    })
      .throw(TransactionError)
      .includes({
        code: ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("Constructor WALLET_UNDEFINED", async () => {
    expect(() => {
      new Transaction({
        // @ts-ignore:
        provider: { poseidon, solMerkleTree: {} },
        params,
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.WALLET_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("Constructor WALLET_RELAYER_INCONSISTENT", async () => {
    const params1 = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      senderSpl: mockPubkey,
      senderSol: mockPubkey,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    expect(() => {
      new Transaction({
        provider: lightProvider,
        params: params1,
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.WALLET_RELAYER_INCONSISTENT,
        functionName: "constructor",
      });
  });

  it("Constructor TX_PARAMETERS_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore:
      new Transaction({
        provider: lightProvider,
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("getProof VERIFIER_IDL_UNDEFINED", async () => {
    expect(() => {
      new Transaction({
        provider: lightProvider,
        // @ts-ignore
        params: {},
      });
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.VERIFIER_IDL_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("getProofInternal PROOF_INPUT_UNDEFINED", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params,
    });
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_INPUT_UNDEFINED,
    );
  });

  it("getAppProof APP_PARAMETERS_UNDEFINED", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params,
    });
    await chai.assert.isRejected(
      tx.getAppProof(),
      TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
    );
  });

  it("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
    let tx = new Transaction({
      provider: {
        // @ts-ignore
        solMerkleTree: {},
        poseidon,
        wallet: lightProvider.wallet,
      },
      params,
    });
    await chai.assert.isRejected(
      tx.getRootIndex(),
      SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
    );
  });

  it("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
    let tx = new Transaction({
      provider: {
        // @ts-ignore
        solMerkleTree: {},
        poseidon,
        wallet: lightProvider.wallet,
      },
      params,
    });
    await chai.assert.isRejected(
      tx.getRootIndex(),
      SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
    );
  });

  it("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
    let tx = new Transaction({
      // @ts-ignore
      provider: lightProvider,
      params,
    });
    // @ts-ignore
    tx.params.assetPubkeysCircuit = undefined;
    expect(() => {
      tx.getIndices(params.inputUtxos);
    })
      .throw(TransactionError)
      .includes({
        code: TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
        functionName: "getIndices",
      });
  });
});

describe("Transaction Functional Tests", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon: any,
    lightProvider: LightProvider,
    deposit_utxo1: Utxo,
    relayer: Relayer,
    keypair,
    paramsDeposit: TransactionParameters,
    paramsWithdrawal: TransactionParameters;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey3, mockPubkey, new anchor.BN(5000));
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      blinding: new anchor.BN(new Array(31).fill(1)),
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    paramsDeposit = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet?.publicKey,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      deposit_utxo1.getCommitment(poseidon),
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        deposit_utxo1.getCommitment(poseidon),
      ),
      0,
    );
    paramsWithdrawal = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      recipientSpl: mockPubkey,
      recipientSol: lightProvider.wallet?.publicKey,
      action: Action.UNSHIELD,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("Functional ", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });
    await tx.compileAndProve();
  });

  it("Functional storage ", async () => {
    const paramsDepositStorage = new TransactionParameters({
      message: Buffer.alloc(928).fill(1),
      inputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      recipientSpl: mockPubkey,
      recipientSol: lightProvider.wallet?.publicKey,
      action: Action.UNSHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      relayer,
    });
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDepositStorage,
    });
    await tx.compileAndProve();
    await tx.getInstructions(tx.params);
  });

  it("getMint ", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });
    let mint = tx.getMint();
    assert.equal(
      mint.toString(),
      hashAndTruncateToCircuit(MINT.toBuffer()).toString(),
    );
    assert.notEqual(mint.toString(), MINT.toString());
  });

  it("getRootIndex Provider Undefined", async () => {
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsDeposit,
    });
    await tx.getRootIndex();
    assert.equal(tx.transactionInputs.rootIndex?.toNumber(), 0);
  });

  it("getIndices", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let mockPubkey = SolanaKeypair.generate().publicKey;
    let lightProvider = await LightProvider.loadMock();

    let deposit_utxo1 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(1), new anchor.BN(2)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const relayer = new Relayer(mockPubkey, mockPubkey, new anchor.BN(5000));

    let params = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      recipientSpl: mockPubkey,
      recipientSol: mockPubkey,
      poseidon,
      action: Action.UNSHIELD,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    let tx = new Transaction({
      provider: lightProvider,
      params,
    });

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

    let deposit_utxo2 = new Utxo({
      poseidon,
      assets: [FEE_ASSET],
      amounts: [new anchor.BN(1)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const indices3 = tx.getIndices([deposit_utxo2]);
    assert.equal(indices3[0][0][0], "1");
    assert.equal(indices3[0][0][1], "0");
    assert.equal(indices3[0][0][2], "0");
    assert.equal(indices3[0][1][0], "0");
    assert.equal(indices3[0][1][1], "0");
    assert.equal(indices3[0][1][2], "0");

    let deposit_utxo3 = new Utxo({
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const indices4 = tx.getIndices([deposit_utxo3]);
    assert.equal(indices4[0][0][0], "0");
    assert.equal(indices4[0][0][1], "0");
    assert.equal(indices4[0][0][2], "0");
    assert.equal(indices4[0][1][0], "0");
    assert.equal(indices4[0][1][1], "0");
    assert.equal(indices4[0][1][2], "0");

    let deposit_utxo4 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(0), new anchor.BN(2)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
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

    let deposit_utxo5 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(2), new anchor.BN(0)],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const indices7 = tx.getIndices([deposit_utxo5]);
    assert.equal(indices7[0][0][0], "1");
    assert.equal(indices7[0][0][1], "0");
    assert.equal(indices7[0][0][2], "0");
    assert.equal(indices7[0][1][0], "0");
    assert.equal(indices7[0][1][1], "1");
    assert.equal(indices7[0][1][2], "0");
  });

  it("getConnectingHash", async () => {
    const relayerConst = new Relayer(AUTHORITY, AUTHORITY, new anchor.BN(5000));
    const paramsStaticEncryptedUtxos = new TransactionParameters({
      inputUtxos: [deposit_utxo1, deposit_utxo1],
      outputUtxos: [deposit_utxo1, deposit_utxo1],
      transactionMerkleTreePubkey: AUTHORITY,
      poseidon,
      recipientSpl: AUTHORITY,
      recipientSol: lightProvider.wallet?.publicKey,
      action: Action.UNSHIELD,
      relayer: relayerConst,
      encryptedUtxos: new Uint8Array(256).fill(1),
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    let txIntegrityHash = await paramsStaticEncryptedUtxos.getTxIntegrityHash(
      poseidon,
    );

    assert.equal(
      txIntegrityHash.toString(),
      "8474219873742569926077283601668996541206408042377172085414034533116551539216",
    );
    assert.equal(
      Transaction.getTransactionHash(
        paramsStaticEncryptedUtxos,
        poseidon,
      ).toString(),
      "18885149309354713641176366184956071391463483813877037254205685046110691645566",
    );
  });

  it("getMerkleProof", async () => {
    let merkleProofsDeposit = Transaction.getMerkleProofs(
      lightProvider,
      paramsDeposit.inputUtxos,
    );
    assert.equal(
      merkleProofsDeposit.inputMerklePathIndices.toString(),
      new Array(2).fill("0").toString(),
    );
    assert.equal(
      merkleProofsDeposit.inputMerklePathElements[0].toString(),
      new Array(18).fill("0").toString(),
    );
    assert.equal(
      merkleProofsDeposit.inputMerklePathElements[1].toString(),
      new Array(18).fill("0").toString(),
    );

    let merkleProofsWithdrawal = Transaction.getMerkleProofs(
      lightProvider,
      paramsWithdrawal.inputUtxos,
    );
    assert.equal(
      merkleProofsWithdrawal.inputMerklePathIndices.toString(),
      new Array(2).fill("0").toString(),
    );

    const constElements = [
      "14522046728041339886521211779101644712859239303505368468566383402165481390632",
      "12399300409582020702502593817695692114365413884629119646752088755594619792099",
      "8395588225108361090185968542078819429341401311717556516132539162074718138649",
      "4057071915828907980454096850543815456027107468656377022048087951790606859731",
      "3743829818366380567407337724304774110038336483209304727156632173911629434824",
      "3362607757998999405075010522526038738464692355542244039606578632265293250219",
      "20015677184605935901566129770286979413240288709932102066659093803039610261051",
      "10225829025262222227965488453946459886073285580405166440845039886823254154094",
      "5686141661288164258066217031114275192545956158151639326748108608664284882706",
      "13358779464535584487091704300380764321480804571869571342660527049603988848871",
      "20788849673815300643597200320095485951460468959391698802255261673230371848899",
      "18755746780925592439082197927133359790105305834996978755923950077317381403267",
      "10861549147121384785495888967464291400837754556942768811917754795517438910238",
      "7537538922575546318235739307792157434585071385790082150452199061048979169447",
      "19170203992070410766412159884086833170469632707946611516547317398966021022253",
      "9623414539891033920851862231973763647444234218922568879041788217598068601671",
      "3060533073600086539557684568063736193011911125938770961176821146879145827363",
      "138878455357257924790066769656582592677416924479878379980482552822708744793",
    ];
    assert.equal(
      merkleProofsWithdrawal.inputMerklePathElements[0].toString(),
      constElements.toString(),
    );

    assert.equal(
      merkleProofsWithdrawal.inputMerklePathElements[1].toString(),
      new Array(18).fill("0").toString(),
    );
  });

  it("getPdaAddresses", async () => {
    const relayerConst = new Relayer(
      AUTHORITY,
      AUTHORITY,
      AUTHORITY,
      new anchor.BN(5000),
    );
    const paramsStaticEncryptedUtxos = new TransactionParameters({
      inputUtxos: [deposit_utxo1, deposit_utxo1],
      outputUtxos: [deposit_utxo1, deposit_utxo1],
      transactionMerkleTreePubkey: AUTHORITY,
      poseidon,
      recipientSpl: AUTHORITY,
      recipientSol: lightProvider.wallet?.publicKey,
      action: Action.UNSHIELD,
      relayer: relayerConst,
      encryptedUtxos: new Uint8Array(256).fill(1),
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    let tx = new Transaction({
      provider: lightProvider,
      params: paramsStaticEncryptedUtxos,
    });
    // @ts-ignore
    tx.transactionInputs.publicInputs = { leaves: [], nullifiers: [] };
    tx.transactionInputs.publicInputs!.outputCommitment = [
      new Array(32).fill(1),
      new Array(32).fill(1),
    ];
    tx.transactionInputs.publicInputs!.inputNullifier = [
      new Array(32).fill(1),
      new Array(32).fill(1),
    ];
    tx.getPdaAddresses();
    const refNullfiers = [
      "A3rueqakAhxjJVUrygVZdpd3wUNUHiGuKy2M7zR7uHDh",
      "A3rueqakAhxjJVUrygVZdpd3wUNUHiGuKy2M7zR7uHDh",
    ];

    const refLeaves = ["6UuSTaJpEemGVuPkmtTiNe7VndXXenWCDU49aTkGSQqY"];
    for (let i = 0; i < 2; i++) {
      assert.equal(
        tx.remainingAccounts?.nullifierPdaPubkeys![i].pubkey.toBase58(),
        refNullfiers[i],
      );
    }
    assert.equal(
      tx.remainingAccounts?.leavesPdaPubkeys![0].pubkey.toBase58(),
      refLeaves[0],
    );
    assert.equal(
      tx.params.accounts.verifierState!.toBase58(),
      "5XAf8s2hi4fx3QK8fm6dgkfXLE23Hy9k1Qo3ew6QqdGP",
    );
  });

  it("APP_PARAMETERS_UNDEFINED", async () => {
    const params = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: mockPubkey,
      poseidon,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });
    expect(() => {
      let tx = new Transaction({
        provider: lightProvider,
        params,
      });
    })
      .to.throw(TransactionError)
      .to.include({
        code: TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("INVALID_VERIFIER_SELECTED", async () => {
    const params = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: mockPubkey,
      poseidon,
      action: Action.SHIELD,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    expect(() => {
      let tx = new Transaction({
        provider: lightProvider,
        params,
        appParams: { mock: "1231", verifierIdl: IDL_VERIFIER_PROGRAM_ZERO },
      });
    })
      .to.throw(TransactionError)
      .to.include({
        code: TransactionErrorCode.INVALID_VERIFIER_SELECTED,
        functionName: "constructor",
      });
  });
});
