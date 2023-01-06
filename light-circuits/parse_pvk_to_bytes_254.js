var ffjavascript = require('ffjavascript');
const {unstringifyBigInts, leInt2Buff} = ffjavascript.utils;
const Scalar = ffjavascript.Scalar;
var fs = require("fs")


async function main() {
  let file = await fs.readFile("verification_key_mainnet.json", async function(err, fd) {
   if (err) {
      return console.error(err);
   }
   console.log("File opened successfully!");
   var mydata = JSON.parse(fd.toString());
   console.log(mydata)

   for (var i in mydata) {
     //console.log(`${i}: ${mydata[i]}`);
     //console.log(i)
     if (i == 'vk_alpha_1') {

       for (var j in mydata[i]) {
         mydata[i][j] = leInt2Buff(unstringifyBigInts(mydata[i][j]), 32).reverse()
       }
     } else if (i == 'vk_beta_2') {
       for (var j in mydata[i]) {
         console.log("mydata[i][j] ", mydata[i][j])

         let tmp = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)).concat(Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32))).reverse()
         console.log("tmp ", tmp);
         mydata[i][j][0] = tmp.slice(0,32)
         mydata[i][j][1] = tmp.slice(32,64)
         console.log("mydata[i][j] ", mydata[i][j])
         // for (var z in mydata[i][j]){
         //   mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32)
         // }
       }
     } else if (i == 'vk_gamma_2') {
       for (var j in mydata[i]) {
         let tmp = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)).concat(Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32))).reverse()
         console.log(`i ${i}, tmp ${tmp}`)
         mydata[i][j][0] = tmp.slice(0,32)
         mydata[i][j][1] = tmp.slice(32,64)

         // for (var z in mydata[i][j]){
         //   mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32)
         // }
       }
     } else if (i == 'vk_delta_2') {
       for (var j in mydata[i]) {
         let tmp = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][0]), 32)).concat(Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][1]), 32))).reverse()
         mydata[i][j][0] = tmp.slice(0,32)
         mydata[i][j][1] = tmp.slice(32,64)

         // for (var z in mydata[i][j]){
         //   mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32)
         // }
       }
     }
     else if (i == 'vk_alphabeta_12') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
           for (var u in mydata[i][j][z]){
             //console.log(mydata[i][j][z][u])
             //console.log(unstringifyBigInts(mydata[i][j][z][u]) == unstringifyBigInts("0x1134F9559674416FA8944B691B7F2F7127D7FEEFC223D06B9803E579D741EBF998577CA406D1ADD6D0872D531B956BD0"))

             mydata[i][j][z][u] = leInt2Buff(unstringifyBigInts(mydata[i][j][z][u]))

           }
         }
       }
     }


     else if (i == 'IC') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
            //console.log(mydata[i][j][z][u])
            console.log(unstringifyBigInts(mydata[i][j][z]))
            // console.log( unstringifyBigInts("0x000101000774525E16DB67472F30D72572668B30199F50E677870A6256924A10E1B984AC306D636B14A71B978FB966EC"))
            //
            // console.log(unstringifyBigInts(mydata[i][j][z]) == unstringifyBigInts("0x1167BA5EEA3212515FFCB9285297A587DD20D56D797611D5D9EB8D2CB5A19DC323DCC23082916ECC86C56592A4DBFCA7"))
            console.log(unstringifyBigInts(mydata[i][j][z]) == unstringifyBigInts("0x279A3A31DB55A7D9E82122ADFD708F5FF0DD33706A0404DD0E6EA06D9B83452E"))
            console.log(unstringifyBigInts(mydata[i][j][z]) == unstringifyBigInts("0x2875717270029F096FEC64D14A540AAD5475725428F790D6D848B32E98A81FBE"))

            if (z == 1) {
              //console.log(leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32))
            }
            mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).reverse()

         }
       }
     }
     // for (var j in mydata[i]) {
     //   for (var z in mydata[i][j]){
     //     console.log(mydata[i][j][z])
     //
     //   }
     // }

     // for (var j in i) {
     //   console.log(j)
     //
     // }

   }
   console.log(mydata)
   let resFile = await fs.openSync("verification_key_bytes_mainnet.rs","w")
   let s = `use groth16_solana::groth16::Groth16Verifyingkey;\n\npub const VERIFYINGKEY: Groth16Verifyingkey =  Groth16Verifyingkey {\n\tnr_pubinputs: ${mydata.IC.length},\n\n`
   s += "\tvk_alpha_g1: [\n"
   for (var j = 0; j < mydata.vk_alpha_1.length -1 ; j++) {
     console.log(typeof(mydata.vk_alpha_1[j]))
     s += "\t\t" + Array.from(mydata.vk_alpha_1[j])/*.reverse().toString()*/ + ",\n"
   }
   s += "\t],\n\n"
   fs.writeSync(resFile,s)
   s = "\tvk_beta_g2: [\n"
   for (var j = 0; j < mydata.vk_beta_2.length -1 ; j++) {
     for (var z = 0; z < 2; z++) {
       s += "\t\t" + Array.from(mydata.vk_beta_2[j][z])/*.reverse().toString()*/ + ",\n"
     }
   }
   s += "\t],\n\n"
   fs.writeSync(resFile,s)
   s = "\tvk_gamme_g2: [\n"
   for (var j = 0; j < mydata.vk_gamma_2.length -1 ; j++) {
     for (var z = 0; z < 2; z++) {
       s += "\t\t" + Array.from(mydata.vk_gamma_2[j][z])/*.reverse().toString()*/ + ",\n"
     }
   }
   s += "\t],\n\n"
   fs.writeSync(resFile,s)

   s = "\tvk_delta_g2: [\n"
   for (var j = 0; j < mydata.vk_delta_2.length -1 ; j++) {
     for (var z = 0; z < 2; z++) {
       s += "\t\t" + Array.from(mydata.vk_delta_2[j][z])/*.reverse().toString()*/ + ",\n"
     }
   }
   s += "\t],\n\n"
   fs.writeSync(resFile,s)
   s = "\tvk_ic: &[\n"
   let x = 0;
   console.log("mydata.IC, ", mydata.IC)
   for (var ic in mydata.IC) {
     s += "\t\t[\n"
     // console.log(mydata.IC[ic])
     for (var j = 0; j < mydata.IC[ic].length - 1 ; j++) {
       s += "\t\t\t" + mydata.IC[ic][j]/*.reverse().toString()*/ + ",\n"
     }
     x++;
     s += "\t\t],\n"
   }
   s += "\t]\n};"

   console.log("Public inputs", x)
   fs.writeSync(resFile,s)
   //console.log(JSON.stringify(mydata))
  // fs.writeFile('verification_key_bytes_mainnet.txt', JSON.stringify(mydata), function(err) {
  //     if (err) {
  //        return console.error(err);
  //     }
  //
  //     // console.log("Data written successfully!");
  //     // console.log("Let's read newly written data");
  //     //
  //     // fs.readFile('verification_key_bytes.txt', function (err, data) {
  //     //    if (err) {
  //     //       return console.error(err);
  //     //    }
  //     //    console.log(JSON.parse(data));
  //     // });
  //     console.log("Pvk written to bytes");
  //   });


 });
 //console.log(Scalar.fromString("0B85DC05397EAC823D25C7C4682A0BE95141A33334C65A857D2491680F972BA4A5D50D9FB71A87E0594E32C02B9324E4"), 32);

  //console.log(leInt2Buff(unstringifyBigInts()))

}


main()
