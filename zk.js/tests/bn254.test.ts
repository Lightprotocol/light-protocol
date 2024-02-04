import { expect } from "chai";
import { BN } from "@coral-xyz/anchor";
import { createBN254, BN254toBase58 } from "../src/utxo/bn254";
import { FIELD_SIZE } from "../src/constants";
import { randomBN } from "../src";

describe("BN254 Tests", () => {
  describe("createBN254", () => {
    it("should create a BN254 instance from a number", () => {
      const number = 123;
      const bn254 = createBN254(number);
      expect(bn254).to.be.instanceOf(BN);
      expect(bn254.toString()).to.equal("123");
    });

    it("should create a BN254 instance from a base58 string", () => {
      const base58String = "2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av";
      const bn254 = createBN254(base58String, "base58");
      expect(bn254).to.be.instanceOf(BN);
      expect(bn254.toString()).to.equal(
        "10784149751864907791693680095463130962913028526988979834148458367799453575891",
      );
    });

    it("should throw an error if the number exceeds <254 bits", () => {
      const largeNumber = FIELD_SIZE; // FIELD_SIZE is the limit
      expect(() => createBN254(largeNumber)).to.throw(
        "Value is too large. Max <254 bits",
      );
      const evenLargerNumber = FIELD_SIZE.addn(1);
      expect(() => createBN254(evenLargerNumber)).to.throw(
        "Value is too large. Max <254 bits",
      );
    });
  });

  describe("BN254toBase58", () => {
    it("should convert a BN254 instance to a base58 string", () => {
      const number =
        "811845991933502754394233624001712393569448066157129948066554739270008360";

      const bn254 = createBN254(number);
      const base58String = BN254toBase58(bn254);
      expect(base58String).to.be.a("string");
      expect(base58String).to.equal(
        "11Qa3SksvZMdPJSQXLMEd6APJscLyg8sbJeGnFedtCF",
      );
      expect([44, 43]).to.include(base58String.length);
      // and convert it back
      const bn254FromBase58 = createBN254(base58String, "base58");
      expect(bn254FromBase58.toString()).to.equal(bn254.toString());
    });
    it("should convert a small BN254 instance to a base58 string", () => {
      const number = "811845991933502754394233624";
      const bn254 = createBN254(number);
      const base58String = BN254toBase58(bn254);
      expect(base58String).to.be.a("string");
      expect([36]).to.include(base58String.length);
      // and convert it back
      const bn254FromBase58 = createBN254(base58String, "base58");
      expect(bn254FromBase58.toString()).to.equal(bn254.toString());
    });
    it("should convert a random BN254 instance to a base58 string", () => {
      const number = new BN(randomBN(), 30, "be");

      const bn254 = createBN254(number);
      const base58String = BN254toBase58(bn254);
      expect(base58String).to.be.a("string");
      expect([44, 43]).to.include(base58String.length);
      // and convert it back
      const bn254FromBase58 = createBN254(base58String, "base58");
      expect(bn254FromBase58.toString()).to.equal(bn254.toString());
    });
  });
});
