const fs = require('fs');

function main() {
    const dir = process.argv[2];
    const name = process.argv[3];

    // check whether program Cargo.toml file exists
    const programDir = `${dir}/programs/${name}/Cargo.toml`;
    if (!fs.existsSync(programDir))
        throw new Error(`Program ${programDir} does not exist.`);
    console.log(`Checked ${programDir} success`);
}

main();