import buildMimc7 from "./mimc7.js";

async function run() {
    const mimc7 = await buildMimc7();

    console.log("IV: "+mimc7.getIV().toString());
}

run().then(()=> {
    process.exit(0);
}, (err) => {
    console.log(err.stack);
    console.log(err.message);
    process.exit(1);
});


