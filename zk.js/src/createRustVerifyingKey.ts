import { execSync } from "child_process";
import { camelToScreamingSnake } from "./convertCase";

const ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const fs = require("fs");
const snarkjs = require("snarkjs");

// type array of circuit inputs properties
type PropertiesObject = {
  inputName: string;
  dimension: number;
  size: number[];
  public?: 0 | 1;
  sumSize?: number;
};

/**
 * 1- Regex matching to filter main signals taken from .sym file.
 * 2- Extract properties: array dimension, size, Public, Private.
 * 3- Read .r1cs file and save the #total of Prv, Pbl inputs as well as outputs.
 * 4- Filter inputs with unique name and max size according to circom signals format.
 */
async function getProofInputsFromSymFile(
  artifactPath: string,
  suffix: string = "",
) {
  // filter inputData array based on the maximum size of nested arrays([0] otherwise)
  function uniqueMaxSize(arr: PropertiesObject[]) {
    const uniqueArr = arr.reduce((acc: PropertiesObject[], cur) => {
      const { inputName, dimension, size } = cur;
      const Public = cur.public;
      const sumSize = size.reduce((a, b) => a + b, 0);

      const idx = acc.findIndex(
        (obj: PropertiesObject) =>
          obj.inputName === inputName && obj.sumSize! < sumSize,
      );

      if (idx === -1) {
        acc.push({ inputName, dimension, size, sumSize, public: Public });
      } else {
        acc[idx] = { inputName, dimension, size, sumSize, public: Public };
      }

      return acc;
    }, []);

    return uniqueArr.reduce((acc: PropertiesObject[], cur) => {
      const idx = acc.findIndex(
        (obj: PropertiesObject) => obj.inputName === cur.inputName,
      );
      if (idx === -1) {
        delete cur.sumSize;
        acc.push(cur);
      }
      return acc;
    }, []);
  }

  // filter signal names from the sym file
  const regex = /main\.(.+)/g;

  let match: any[] | null;
  let keys: any[] = [];
  const symText = fs.readFileSync(`${artifactPath}${suffix}.sym`, "utf-8");
  while ((match = regex.exec(symText)) !== null) {
    keys.push(match[1]);
  }

  const arr: PropertiesObject[] = [];

  keys.map((name) => {
    const dimension = (name.match(/\[/g) || []).length;
    const inputName = dimension === 0 ? name : name.slice(0, name.indexOf("["));
    const size =
      dimension === 0
        ? [0]
        : (name.match(/\[(.*?)\]/g) || [])
            .map((m) => m.replace(/\[|\]/g, ""))
            .map((n) => parseInt(n) + 1);

    arr.push({ inputName, dimension, size });
  });

  // Retrieve the number of outputs as well as the number of private and public inputs from the R1CS file
  const r1cs = await snarkjs.r1cs.exportJson(`${artifactPath}${suffix}.r1cs`);
  const nOut = r1cs.nOutputs;
  const nPub = r1cs.nPubInputs;
  const nPrv = r1cs.nPrvInputs;
  const total = nOut + nPub + nPrv;

  // Retrieve the main inputs and outputs and select unique input names
  const inputs_arr = arr.slice(0, total);

  for (let i = 0; i < total; i++) {
    if (i < nOut + nPub) arr[i].public = 1;
    else arr[i].public = 0;
  }
  const marr = uniqueMaxSize(inputs_arr);

  const inputsNum = marr.length;
  const inputs = marr.slice(0, inputsNum);

  return inputs;
}

function createStringRsIdlAccountStruct(
  preparedInputs: PropertiesObject[],
  circuitName: string,
  prefix: string = "ZK",
  isInstructionData: boolean = false,
) {
  function camelToSnakeCase(str: string) {
    return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
  }

  function buildRustType(
    dimension: number,
    size: number[],
    isInstructionData: boolean = false,
  ): string {
    // the base type is a 254-bit bigint; thus, a 32-byte array
    if (dimension === 0 && isInstructionData) {
      return "[u8; 32]";
    }
    if (dimension === 0) {
      return "u8";
    }

    const rustType = buildRustType(
      dimension - 1,
      size.slice(1),
      isInstructionData,
    );
    return `[${rustType};${size[0]}]`;
  }

  ///  MAIN FUNCTION START
  // parse the inputs output into a rust struct as a program account
  let structDefinition = `\n#[account]\npub struct ${prefix}${circuitName} {\n`;

  preparedInputs.forEach((input) => {
    const { inputName, dimension, size } = input;
    const rustType = buildRustType(dimension, size, isInstructionData);
    const inputName_snake = camelToSnakeCase(inputName);
    structDefinition += `${inputName_snake}: ${rustType},\n`;
  });

  structDefinition += "}";

  return structDefinition;
}

async function createVerifyingKeyRsFile(
  vKeyJsonPath: string,
  paths: string[],
  appendingString: string,
  circuitName: string,
) {
  await fs.readFile(
    vKeyJsonPath,
    async function (err: Error | null, fd: Buffer) {
      if (err) {
        return console.error(err);
      }
      const mydata = JSON.parse(fd.toString());

      for (const i in mydata) {
        if (i == "vk_alpha_1") {
          for (const j in mydata[i]) {
            mydata[i][j] = leInt2Buff(
              unstringifyBigInts(mydata[i][j]),
              32,
            ).reverse();
          }
        } else if (i == "vk_beta_2") {
          for (const j in mydata[i]) {
            const tmp = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)
            )
              .concat(
                Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32)),
              )
              .reverse();
            mydata[i][j][0] = tmp.slice(0, 32);
            mydata[i][j][1] = tmp.slice(32, 64);
          }
        } else if (i == "vk_gamma_2") {
          for (const j in mydata[i]) {
            const tmp = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)
            )
              .concat(
                Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32)),
              )
              .reverse();
            mydata[i][j][0] = tmp.slice(0, 32);
            mydata[i][j][1] = tmp.slice(32, 64);
          }
        } else if (i == "vk_delta_2") {
          for (const j in mydata[i]) {
            const tmp = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)
            )
              .concat(
                Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32)),
              )
              .reverse();
            mydata[i][j][0] = tmp.slice(0, 32);
            mydata[i][j][1] = tmp.slice(32, 64);
          }
        } else if (i == "vk_alphabeta_12") {
          for (const j in mydata[i]) {
            for (const z in mydata[i][j]) {
              for (const u in mydata[i][j][z]) {
                mydata[i][j][z][u] = leInt2Buff(
                  unstringifyBigInts(mydata[i][j][z][u]),
                );
              }
            }
          }
        } else if (i == "IC") {
          for (const j in mydata[i]) {
            for (const z in mydata[i][j]) {
              mydata[i][j][z] = leInt2Buff(
                unstringifyBigInts(mydata[i][j][z]),
                32,
              ).reverse();
            }
          }
        }
      }

      for (const path of paths) {
        const resFile = await fs.openSync(path, "w");

        let s = `use groth16_solana::groth16::Groth16Verifyingkey;\nuse anchor_lang::prelude::*;\n\npub const VERIFYINGKEY_${camelToScreamingSnake(
          circuitName,
        )}: Groth16Verifyingkey = Groth16Verifyingkey {\n\tnr_pubinputs: ${
          mydata.IC.length - 1
        },\n`;
        s += "\tvk_alpha_g1: [\n";
        for (let j = 0; j < mydata.vk_alpha_1.length - 1; j++) {
          s += "\t\t" + Array.from(mydata.vk_alpha_1[j]) + ",\n";
        }
        s += "\t],\n\n";
        fs.writeSync(resFile, s);
        s = "\tvk_beta_g2: [\n";
        for (let j = 0; j < mydata.vk_beta_2.length - 1; j++) {
          for (let z = 0; z < 2; z++) {
            s += "\t\t" + Array.from(mydata.vk_beta_2[j][z]) + ",\n";
          }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile, s);
        s = "\tvk_gamme_g2: [\n";
        for (let j = 0; j < mydata.vk_gamma_2.length - 1; j++) {
          for (let z = 0; z < 2; z++) {
            s += "\t\t" + Array.from(mydata.vk_gamma_2[j][z]) + ",\n";
          }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile, s);

        s = "\tvk_delta_g2: [\n";
        for (let j = 0; j < mydata.vk_delta_2.length - 1; j++) {
          for (let z = 0; z < 2; z++) {
            s += "\t\t" + Array.from(mydata.vk_delta_2[j][z]) + ",\n";
          }
        }
        s += "\t],\n\n";
        fs.writeSync(resFile, s);
        s = "\tvk_ic: &[\n";

        for (const ic in mydata.IC) {
          s += "\t\t[\n";
          for (let j = 0; j < mydata.IC[ic].length - 1; j++) {
            s += "\t\t\t" + mydata.IC[ic][j] + ",\n";
          }
          s += "\t\t],\n";
        }
        s += "\t]\n};";
        s += appendingString;

        fs.writeSync(resFile, s);
        fs.closeSync(resFile);

        execSync(`rustfmt ${path}`);
      }
    },
  );
}

export async function createVerifyingkeyRsFileArgv() {
  const nrInputs = process.argv[2];
  if (!nrInputs) {
    throw new Error("Circuit nrInputs is not specified!");
  }

  let program: string;
  const paths: string[] = [];
  let vKeyJsonPath: string;
  let vKeyRsPath: string;
  let circuitName: string;
  let artifactPath: string = "";
  if (nrInputs == "app") {
    program = `${process.argv[3]}`;
    vKeyJsonPath = "./verifyingkey.json";
    vKeyRsPath = "./programs/" + program + "/src/verifying_key.rs";
    circuitName = process.argv[4] ? `${process.argv[4]}` : "appTransaction";
  } else {
    if (nrInputs == "2") {
      program = "verifier_program_zero";
      const program_storage = "verifier_program_storage";
      const vKeyRsPath_storage =
        "../../system-programs/programs/" +
        program_storage +
        "/src/verifying_key.rs";
      paths.push(vKeyRsPath_storage);
    } else if (nrInputs == "10") {
      program = "verifier_program_one";
    } else if (nrInputs == "4") {
      program = "verifier_program_two";
    } else {
      throw new Error("invalid nr of inputs");
    }
    vKeyJsonPath = "./verification_key_mainnet" + nrInputs + ".json";
    vKeyRsPath =
      "../../system-programs/programs/" + program + "/src/verifying_key.rs";
    circuitName = "transaction" + process.argv[3];
    artifactPath = "../../zk.js/build-circuits/transaction" + process.argv[3];
  }
  await createVerifyingkeyRsFile(
    program,
    paths,
    vKeyJsonPath,
    vKeyRsPath,
    circuitName,
    artifactPath,
  );
}

export async function createVerifyingkeyRsFile(
  _program: string,
  paths: string[],
  vKeyJsonPath: string,
  vKeyRsPath: string,
  circuitName: string,
  artifactPath: string,
  suffix: string = "",
) {
  if (!vKeyRsPath)
    throw new Error("Undefined output path for the verifying_key.rs file!");
  paths.push(vKeyRsPath);

  const ProofInputs: PropertiesObject[] = await getProofInputsFromSymFile(
    artifactPath,
    suffix,
  );
  const PublicInputs = ProofInputs.filter(
    (ProofInputs) => ProofInputs.public === 1,
  );
  let appendingStrings = createStringRsIdlAccountStruct(
    ProofInputs,
    circuitName + "ProofInputs",
    "ZK",
    false,
  );
  appendingStrings += createStringRsIdlAccountStruct(
    PublicInputs,
    circuitName + "PublicInputs",
    "ZK",
    false,
  );
  const circuitNameUpperCamelCase =
    circuitName.charAt(0).toUpperCase() + circuitName.slice(1);
  appendingStrings += createStringRsIdlAccountStruct(
    PublicInputs,
    `InstructionDataLightInstruction${circuitNameUpperCamelCase}Second`,
    "",
    true,
  );

  // Write verifying_key.rs file for the circuit
  await createVerifyingKeyRsFile(
    vKeyJsonPath,
    paths,
    appendingStrings,
    circuitName,
  );
}
