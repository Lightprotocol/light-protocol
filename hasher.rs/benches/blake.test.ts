import { blake2b } from "@noble/hashes/blake2b";
import { Bench } from 'tinybench';
const bench = new Bench({ time: 1000 });

bench
.add('@noble/hashes/blake2b', async () => {
    blake2b.create({ dkLen: 32 }).update("").digest();
})

await bench.run();

console.table(bench.table());