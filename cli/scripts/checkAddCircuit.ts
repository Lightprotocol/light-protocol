const fs = require('fs');
import { toSnakeCase } from "@lightprotocol/zk.js";
import { toCamelCase } from "../src/psp-utils";

function main() {
    const dir = process.argv[2];
    const name = process.argv[3];
    const circom = process.argv.length == 5 ? process.argv[4]: false;
    console.log(`Checking ${name} in ${dir}  ${circom}`);
    if (circom) {
        // check whether circuit file exists
        const circuitFile = `${dir}/circuits/${name}/${toSnakeCase(name)}.circom`;
        const circuitMainFile = `${dir}/circuits/${name}/${toCamelCase(name)}Main.circom`;
        if (!fs.existsSync(circuitFile))
            throw new Error(`Circuit file ${circuitFile} does not exist.`);
        if (!fs.existsSync(circuitMainFile))
            throw new Error(`Circuit main file ${circuitMainFile} does not exist.`);
    } else {
        // check whether circuit file exists
        const circuitFile = `${dir}/circuits/${name}/${toSnakeCase(name)}.light`;
        if (!fs.existsSync(circuitFile))
            throw new Error(`Circuit file ${circuitFile} does not exist.`);
    }
}

main();