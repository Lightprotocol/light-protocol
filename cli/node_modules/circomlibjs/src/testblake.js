import createBlakeHash from 'blake-hash';
import blake2b from "blake2b";

const msg = (new TextEncoder()).encode("blake256");
const msgB = Buffer.from(msg)


const toHexString = bytes =>
  bytes.reduce((str, byte) => str + byte.toString(16).padStart(2, '0'), '');

const h1 = createBlakeHash('blake256').digest();

const h2 = blake2b(64).digest();


console.log(toHexString(h1));
console.log(toHexString(h2));
