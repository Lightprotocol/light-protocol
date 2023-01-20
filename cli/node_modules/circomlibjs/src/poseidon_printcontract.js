import {createCode} from "./poseidon_gencontract.js";

if (process.argv.length != 3) {
    console.log("Usage: node poseidon_gencontract.js [numberOfInputs]");
    process.exit(1);
}

const nInputs = Number(process.argv[2]);

console.log(nInputs);

console.log(createCode(nInputs));

