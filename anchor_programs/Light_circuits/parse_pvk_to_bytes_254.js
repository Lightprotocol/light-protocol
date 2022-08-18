var ffjavascript = require('ffjavascript');
const {unstringifyBigInts, leInt2Buff} = ffjavascript.utils;
const Scalar = ffjavascript.Scalar;
var fs = require("fs")


async function main() {
  let file = await fs.readFile("verification_key_mainnet.json", function(err, fd) {
   if (err) {
      return console.error(err);
   }
   console.log("File opened successfully!");
   var mydata = JSON.parse(fd.toString());
   //console.log(mydata.vk_delta_2)

   for (var i in mydata) {
     //console.log(`${i}: ${mydata[i]}`);
     //console.log(i)
     if (i == 'vk_alpha_1') {

       for (var j in mydata[i]) {
         //console.log("[ " + leInt2Buff(unstringifyBigInts(mydata[i][j], 32)) + " ]")

         mydata[i][j] = leInt2Buff(unstringifyBigInts(mydata[i][j]), 32).toString()
       }
     } else if (i == 'vk_beta_2') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
           //console.log(mydata[i][j][z])
           mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).toString()


         }
       }
     } else if (i == 'vk_gamma_2') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
           //console.log(mydata[i][j][z])
           mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).toString()

         }
       }
     } else if (i == 'vk_delta_2') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
           //console.log(mydata[i][j][z])
           mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).toString()

         }
       }
     }
     else if (i == 'vk_alphabeta_12') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
           for (var u in mydata[i][j][z]){
             //console.log(mydata[i][j][z][u])
             //console.log(unstringifyBigInts(mydata[i][j][z][u]) == unstringifyBigInts("0x1134F9559674416FA8944B691B7F2F7127D7FEEFC223D06B9803E579D741EBF998577CA406D1ADD6D0872D531B956BD0"))

             //mydata[i][j][z][u] = leInt2Buff(unstringifyBigInts(mydata[i][j][z][u])).toString()

           }
         }
       }
     }


     else if (i == 'IC') {
       for (var j in mydata[i]) {
         for (var z in mydata[i][j]){
            //console.log(mydata[i][j][z][u])
            // console.log(unstringifyBigInts(mydata[i][j][z]))
            // console.log( unstringifyBigInts("0x000101000774525E16DB67472F30D72572668B30199F50E677870A6256924A10E1B984AC306D636B14A71B978FB966EC"))
            //
            // console.log(unstringifyBigInts(mydata[i][j][z]) == unstringifyBigInts("0x1167BA5EEA3212515FFCB9285297A587DD20D56D797611D5D9EB8D2CB5A19DC323DCC23082916ECC86C56592A4DBFCA7"))
            //console.log(unstringifyBigInts(mydata[i][j][z]) == unstringifyBigInts("0x0021D8D50774525E16DB67472F30D72572668B30199F50E677870A6256924A10E1B984AC306D636B14A71B978FB966EC"))

            if (z == 1) {
              //console.log(leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32))
            }
            mydata[i][j][z] = leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32).toString()

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
   //console.log(JSON.stringify(mydata))
  fs.writeFile('verification_key_bytes_mainnet.txt', JSON.stringify(mydata), function(err) {
      if (err) {
         return console.error(err);
      }

      // console.log("Data written successfully!");
      // console.log("Let's read newly written data");
      //
      // fs.readFile('verification_key_bytes.txt', function (err, data) {
      //    if (err) {
      //       return console.error(err);
      //    }
      //    console.log(JSON.parse(data));
      // });
      console.log("Pvk written to bytes");
    });

 });
 //console.log(Scalar.fromString("0B85DC05397EAC823D25C7C4682A0BE95141A33334C65A857D2491680F972BA4A5D50D9FB71A87E0594E32C02B9324E4"), 32);

  //console.log(leInt2Buff(unstringifyBigInts()))

}


main()
