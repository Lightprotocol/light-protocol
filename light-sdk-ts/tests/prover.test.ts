import { assert, expect } from "chai";
import { Prover } from "../src/transaction/prover";
import { Idl } from "@coral-xyz/anchor";

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
  VerifierZero,
  TransactionErrorCode,
  Action,
  Relayer,
  VerifierTwo,
  VerifierOne,
  AUTHORITY,
  TransactionError,
  ProviderErrorCode,
  SolMerkleTreeErrorCode,
  Utxo,
  Account,
  MerkleTree,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_TWO

} from "../src";
import { log } from "console";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const verifiers = [new VerifierZero(), new VerifierOne(), new VerifierTwo()];

describe("Test Prover Functional", () => {
    let seed32 = new Uint8Array(32).fill(1).toString();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
  
    let mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey1 = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;
    let poseidon,
      lightProvider: LightProvider,
      deposit_utxo1,
      outputUtxo,
      relayer,
      keypair,
      paramsDeposit,
      paramsWithdrawal;
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
        blinding: new anchor.BN(new Array(31).fill(1)),
      });
      paramsDeposit = new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        transactionMerkleTreePubkey: mockPubkey2,
        verifier: new VerifierZero(),
        lookUpTable: lightProvider.lookUpTable,
        poseidon,
        senderSpl: mockPubkey,
        senderSol: lightProvider.wallet?.publicKey,
        action: Action.SHIELD,
        transactionNonce: 0,
        verifierIdl: IDL_VERIFIER_PROGRAM_ZERO
        
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
        verifier: new VerifierZero(),
        poseidon,
        recipientSpl: mockPubkey,
        recipientSol: lightProvider.wallet?.publicKey,
        action: Action.UNSHIELD,
        relayer,
        transactionNonce: 0,
        verifierIdl: IDL_VERIFIER_PROGRAM_ZERO
      });
    });

    it("prover functional test1", async () => {
      let tx = new Transaction({
          provider: lightProvider,
          params: paramsDeposit,
      });
      await tx.compile();
      await tx.getProof();
      
      await tx.getRootIndex();
      tx.getPdaAddresses();
      await tx.getInstructions();

    });

    it("prover functional compileAndProve test", async () => {
      let tx = new Transaction({
          provider: lightProvider,
          params: paramsDeposit,
      });
      await tx.compileAndProve();
    })
   
    it("test compliance of parsedPublicInputs array of Verifier and Prover class", async() => {
      let tx = new Transaction({
        provider: lightProvider,
        params: paramsDeposit,
      });

      await tx.compile();
      const prover = new Prover(
        tx.params.verifierIdl as Idl,
        tx.firstPath as string
      );
      await prover.addProofInputs(tx.proofInput);
      await prover.fullProve();  
      await tx.getProof();
      
      const publicInputsBytes = prover.parseToBytesArray(prover.publicInputs);
      const { unstringifyBigInts, leInt2Buff } = require("ffjavascript").utils;
      const publicInputsJson = JSON.stringify(prover.publicInputs, null, 1);
    
      var publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
      var publicInputsBytesVerifier = new Array<Array<number>>();
      for (var i in publicInputsBytesJson) {
        let ref: Array<number> = Array.from([
          ...leInt2Buff(unstringifyBigInts(publicInputsBytesJson[i]), 32),
        ]).reverse();
        publicInputsBytesVerifier.push(ref);
      }

      expect(publicInputsBytes).to.deep.equal(publicInputsBytesVerifier);
    })

    it("prover functional test2", async () => {
        let tx = new Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        await tx.compile();
        const prover = new Prover(
          tx.params.verifierIdl as Idl,
          tx.firstPath as string
        );
        await prover.addProofInputs(tx.proofInput);
        await prover.fullProve();
        
        await tx.getProof();
       
        // assert compliance of publicInputsBytes of prover and verifier class output
        const publicInputsBytes = prover.parseToBytesArray(prover.publicInputs);
        const parsedPublicInputsObj = tx.params.verifier.parsePublicInputsFromArray(publicInputsBytes);
        const proverParsedPublicInputsObj = prover.parsePublicInputsFromArray(publicInputsBytes);
        expect(tx.transactionInputs.publicInputs).to.deep.equal(parsedPublicInputsObj);
        
        expect(parsedPublicInputsObj.root).to.deep.equal(proverParsedPublicInputsObj.root);
        expect(parsedPublicInputsObj.publicAmountSpl).to.deep.equal(proverParsedPublicInputsObj.publicAmountSpl);
        expect(parsedPublicInputsObj.publicAmountSol).to.deep.equal(proverParsedPublicInputsObj.publicAmountSol);
        expect(parsedPublicInputsObj.txIntegrityHash).to.deep.equal(proverParsedPublicInputsObj.txIntegrityHash);
        expect(parsedPublicInputsObj.publicMintPubkey).to.deep.equal(proverParsedPublicInputsObj.publicMintPubkey);
        expect(parsedPublicInputsObj.outputCommitment, 'outputCommitment').to.deep.equal(proverParsedPublicInputsObj.outputCommitment);
        expect(parsedPublicInputsObj.inputNullifier, 'nullifiers').to.deep.equal(proverParsedPublicInputsObj.inputNullifier);
        
        expect(parsedPublicInputsObj).to.deep.equal(proverParsedPublicInputsObj);

        await tx.getRootIndex();
        await tx.getPdaAddresses();
        await tx.getInstructions();
    });

})