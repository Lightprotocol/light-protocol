import buildPedersenHash from "./pedersenhash.js";

async function run() {
    const pedersenHash = await buildPedersenHash();

    let nBases;
    if (typeof process.argv[2] != "undefined") {
        nBases = parseInt(process.argv[2]);
    } else {
        nBases = 5;
    }

    let baseHash;
    if (typeof process.argv[3] != "undefined") {
        baseHash = process.argv[3];
    } else {
        baseHash = "blake";
    }

    for (let i=0; i < nBases; i++) {
        const p = pedersenHash.getBasePoint(i);
        console.log(`[${pedersenHash.babyJub.F.toString(p[0])},${pedersenHash.babyJub.F.toString(p[1])}]`);
    }


}

run().then(()=> {
    process.exit(0);
}, (err) => {
    console.log(err.stack);
    console.log(err.message);
    process.exit(1);
});

