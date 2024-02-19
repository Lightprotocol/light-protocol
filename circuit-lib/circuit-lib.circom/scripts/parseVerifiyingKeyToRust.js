const ffjavascript = require('ffjavascript');
const {unstringifyBigInts, leInt2Buff} = ffjavascript.utils;
let fs = require("fs");
const process = require('process');

async function main() {
    const inputPath = process.argv[2];
    if (!inputPath) {
        throw new Error("inputPath not specified");
    }

    let outputPath = "";
    if (process.argv[3]) {
        outputPath += process.argv[3] +"/";
    }

    await fs.readFile(inputPath, async function(err, fd) {
        if (err) {
            return console.error(err);
        }
        console.log("File opened successfully!");
        let mydata = JSON.parse(fd.toString());

        for (let i in mydata) {
            if (i === 'vk_alpha_1') {

                for (let j in mydata[i]) {
                    mydata[i][j] = leInt2Buff(unstringifyBigInts(mydata[i][j]), 32).reverse()
                }
            } else if (i === 'vk_beta_2' || i === 'vk_gamma_2' || i === 'vk_delta_2') {
                for (let j in mydata[i]) {
                    let tmp = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)).concat(Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32))).reverse()
                    mydata[i][j][0] = tmp.slice(0,32)
                    mydata[i][j][1] = tmp.slice(32,64)
                }
            }
            else if (i === 'vk_alphabeta_12') {
                for (let j in mydata[i]) {
                    for (let z in mydata[i][j]){
                        for (let u in mydata[i][j][z]){
                            mydata[i][j][z][u] = leInt2Buff(unstringifyBigInts(mydata[i][j][z][u]));
                        }
                    }
                }
            }
            else if (i === 'IC') {
                for (let j in mydata[i]) {
                    for (let z in mydata[i][j]) {
                        mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).reverse();
                    }
                }
            }
        }
        let resFile = fs.openSync(outputPath + "mod.rs", "w");
        let s = `use groth16_solana::groth16::Groth16Verifyingkey;\n\npub const VERIFYINGKEY: Groth16Verifyingkey =  Groth16Verifyingkey {\n\tnr_pubinputs: ${mydata.IC.length},\n\n`;
        s += "\tvk_alpha_g1: [\n";
        for (let j = 0; j < mydata.vk_alpha_1.length -1 ; j++) {
            s += "\t\t" + Array.from(mydata.vk_alpha_1[j]) + ",\n"
        }
        s += "\t],\n\n";
        fs.writeSync(resFile,s);
        s = "\tvk_beta_g2: [\n"
        for (let j = 0; j < mydata.vk_beta_2.length -1 ; j++) {
            for (let z = 0; z < 2; z++) {
                s += "\t\t" + Array.from(mydata.vk_beta_2[j][z]) + ",\n";
            }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile,s);
        s = "\tvk_gamme_g2: [\n"
        for (let j = 0; j < mydata.vk_gamma_2.length -1 ; j++) {
            for (let z = 0; z < 2; z++) {
                s += "\t\t" + Array.from(mydata.vk_gamma_2[j][z]) + ",\n";
            }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile,s)

        s = "\tvk_delta_g2: [\n"
        for (let j = 0; j < mydata.vk_delta_2.length -1 ; j++) {
            for (let z = 0; z < 2; z++) {
                s += "\t\t" + Array.from(mydata.vk_delta_2[j][z]) + ",\n";
            }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile,s);
        s = "\tvk_ic: &[\n";
        let x = 0;

        for (let ic in mydata.IC) {
            s += "\t\t[\n";
            for (let j = 0; j < mydata.IC[ic].length - 1 ; j++) {
                s += "\t\t\t" + mydata.IC[ic][j] + ",\n";
            }
            x++;
            s += "\t\t],\n";
        }
        s += "\t]\n};"

        fs.writeSync(resFile,s);
    });
}


main()