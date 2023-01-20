import * as fs from "fs";
import * as light from "light-sdk";

const saveUser = () => {
  const signature = `23232323232323`;
  fs.writeFile("signature.txt", signature, function (err: any) {
    if (err) throw err;
    console.log("signature cached");
  });

  const decryptedUtxos = [{ test: "testString" }, 232323, "string"];
  fs.writeFile(
    "utxos.txt",
    JSON.stringify(decryptedUtxos),
    function (err: any) {
      if (err) throw err;
      console.log("decrypted utxos cached");
    }
  );
};

const readUser = () => {
  // read secret

  let signature;
  let decryptedUtxos = [];
  try {
    fs.readFile("signature.txt", "utf8", function (err: any, data: any) {
      if (err) throw err;
      console.log(data);
      signature = data;
    });
  } catch (e) {
    console.log(e);
  }
  try {
    fs.readFile("utxos.txt", "utf8", function (err: any, data: any) {
      if (err) throw err;
      console.log(JSON.parse(data));
      decryptedUtxos = JSON.parse(data);
    });
  } catch (e) {
    console.log(e);
  }

  // read encrypted user utxos (indices good enough!)
};

readUser();
console.log("save ?");
saveUser();
