import * as fs from "fs";
import * as light from "light-sdk";

const saveUser = () => {
  fs.writeFile(
    "secret.txt",
    `private key : 121323232322323232323
    
public key: 2323`,
    function (err: any) {
      if (err) throw err;
      console.log("new user created. Back up your secret.txt file!");
    }
  );
};

const readUser = () => {
  fs.readFile("secret.txt", "utf8", function (err: any, data: any) {
    if (err) throw err;
    console.log(data);
  });
};

saveUser();
readUser();
