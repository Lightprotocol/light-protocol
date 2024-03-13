import fs = require('fs');
import { toSnakeCase, toCamelCase } from "@lightprotocol/zk.js";

function main() {
    const dir = process.argv[2];
    const name = process.argv[3];
    const programName = process.argv[4];
    const circom = process.argv.length == 6 ? process.argv[5]: false;
    console.log(`Checking ${name} in ${dir} checking ${circom ? "circom" : "light"} file`);
    if (circom) {
        // check whether circuit file exists
        const circuitFile = `${dir}/circuits/${programName}/${name}/${toSnakeCase(name)}.circom`;
        console.log(`Checking ${circuitFile}`);
        const circuitMainFile = `${dir}/circuits/${programName}/${name}/${toCamelCase(name)}Main.circom`;
        if (!fs.existsSync(circuitFile))
            throw new Error(`Circuit file ${circuitFile} does not exist.`);
        if (!fs.existsSync(circuitMainFile))
            throw new Error(`Circuit main file ${circuitMainFile} does not exist.`);
    } else {
        // check whether circuit file exists
        const circuitFile = `${dir}/circuits/${programName}/${name}/${toSnakeCase(name)}.light`;
        if (!fs.existsSync(circuitFile))
            throw new Error(`Circuit file ${circuitFile} does not exist.`);
    }
}

main();
