import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import { PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import {BN} from "@coral-xyz/anchor";
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
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_TWO,
  IDL_VERIFIER_PROGRAM_STORAGE,
  MESSAGE_MERKLE_TREE_KEY,
  convertAndComputeDecimals,
  generateRandomTestAmount,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

// describe("Transaction Error Tests", () => {
//     it("decimal conversion functional", ()=> {
//         const publicAmountSpl = 2.3;
//         try {
//             convertAndComputeDecimals(publicAmountSpl, new anchor.BN(10));
//         } catch (error) {
//             console.log(error);
//         }

//     })
// })
// import { BN } from 'bn.js';
// import { expect } from 'chai';
// import { convertAndComputeDecimals } from './yourModule'; // replace with your actual module

describe('convertAndComputeDecimals', () => {

  it.skip("random test", () => {
    const  getRandomElement = () => {
      const randomIndex = parseInt(Math.floor(Math.random() * 7).toString());
      return randomIndex;
    }
    for (var i = 0; i < 100000; i++) {
      var decimalsNumber = new BN(getRandomElement());
      console.log("decimals ", decimalsNumber);
      var decimals = new BN(10).pow(new BN(decimalsNumber))
      const amount = generateRandomTestAmount(0,1000_000_000, decimalsNumber.toNumber());
      console.log("amount ", amount);
      console.log("decimals ", decimals.toString());

      const result = convertAndComputeDecimals(amount, decimals);
      expect(result.toString()).to.equal((Math.round(amount * decimals.toNumber())).toString());
    }
  });

  it('should correctly convert number (integer) values', () => {
    const amount = 3;
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('30');
  });

  it('should correctly convert number (float) values', () => {
    const amount = 2.5;
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('25');
  });

  it('should correctly convert number (float) values', () => {
    const amount = 54.08;
    const decimals = new BN(100);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('5408');
  });

  it('should correctly convert string values', () => {
    const amount = '4';
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('40');
  });

  it('should correctly convert BN values', () => {
    const amount = new BN(5);
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('50');
  });

  it('should correctly handle zero amount', () => {
    const amount = 0;
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('0');
  });

  it('should correctly handle zero decimals', () => {
    const amount = 5;
    const decimals = new BN(0);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('5');
  });
  
  it('should throw an error for negative decimals', () => {
    const amount = 5;
    const decimals = new BN(-10);
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw();
  });
  
  it('should correctly handle very large amount', () => {
    const amount = 1e18; // One quintillion
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('10000000000000000000');
  });
  
  it('should correctly handle max u64 amount', () => {
    const amount = new BN("18446744073709551615"); // max u64
    const decimals = new BN(0);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('18446744073709551615');
  });
  
  it('should throw because the output is still a float', () => {
    const amount = 0.01; // One hundred-thousandth
    const decimals = new BN(10);
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw();
  });
  
  it('should correctly handle string with decimal points', () => {
    const amount = '2.5';
    const decimals = new BN(10);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('25');
  });
  
  it('should throw an error for invalid string', () => {
    const amount = 'invalid';
    const decimals = new BN(10);
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw();
  });
  
  it('should throw an error for null amount', () => {
    const amount = null;
    const decimals = new BN(10);
    // @ts-ignore: ignore for testing
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw();
  });
  
  it('should throw an error for undefined amount', () => {
    const amount = undefined;
    const decimals = new BN(10);
    // @ts-ignore: ignore for testing
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw();
  });

  it('should throw an error for negative amount', () => {
    const amount = -3;
    const decimals = new BN(10);
    expect(() => convertAndComputeDecimals(amount, decimals)).to.throw('Negative amounts are not allowed.');
  });
  
  
});
