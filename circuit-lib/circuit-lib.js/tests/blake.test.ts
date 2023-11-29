import { assert } from "chai";
import { it } from "mocha";
const createBlakeHash = require("blake-hash");
const { blake2b } = require("@noble/hashes/blake2b");
describe("Blake libs comparison", () => {
    it("noble/blake vs blake-hash", async () => {
        const b2params = { dkLen: 32 };
        const input = "000";
        const a = new Uint8Array(createBlakeHash("blake512").update(input).digest().slice(0, 32));
        const b = blake2b.create(b2params).update(input).digest();

        assert.equal(a.length, b.length);
        assert.equal(a, b);
    });

});
