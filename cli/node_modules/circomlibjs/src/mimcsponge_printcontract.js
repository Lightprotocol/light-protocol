import {createCode} from "./mimcsponge_gencontract.js";

const SEED = "mimcsponge";

let nRounds;
if (typeof process.argv[2] != "undefined") {
    nRounds = parseInt(process.argv[2]);
} else {
    nRounds = 220;
}

console.log(createCode(SEED, nRounds));

