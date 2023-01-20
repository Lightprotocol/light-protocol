import buildMimcSponge from "./mimcsponge.js";

async function run() {
    const mimcsponge = await buildMimcSponge();
    const nRounds = 220;
    let S = "[\n";
    const cts = mimcsponge.getConstants();
    for (let i=0; i<nRounds; i++) {
        S = S + mimcsponge.F.toString(cts[i]);
        if (i<nRounds-1) S = S + ",";
        S=S+"\n";
    }
    S = S + "]\n";
    
    console.log(S);
}

run().then(()=> {
    process.exit(0);
}, (err) => {
    console.log(err.stack);
    console.log(err.message);
    process.exit(1);
});