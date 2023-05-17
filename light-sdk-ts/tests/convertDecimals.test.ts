import { expect } from "chai";
import {BN} from "@coral-xyz/anchor";
import { it } from "mocha";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import {
  convertAndComputeDecimals,
  generateRandomTestAmount,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

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
    const amount = "2.5";
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
    const amount = new BN(0);
    const decimals = new BN(100);
    const result = convertAndComputeDecimals(amount, decimals);
    expect(result.toString()).to.equal('0');
  });

  it('should handle zero decimals correctly', () => {
    const amount = "5";
    const decimals = new BN(1);
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
    const decimals = new BN(1);
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
