import buildMimc7 from "./mimc7.js";

async function run() {
    const mimc7 = await buildMimc7();
    const nRounds = 91;
    let S = "[\n";
    const cts = mimc7.getConstants();
    for (let i=0; i<nRounds; i++) {
        S = S + mimc7.F.toString(cts[i]);
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

