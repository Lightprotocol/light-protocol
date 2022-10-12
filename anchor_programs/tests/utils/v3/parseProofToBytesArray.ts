var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
export const parseProofToBytesArray = function (data) {
  var mydata = JSON.parse(data.toString());
  for (var i in mydata) {
    if (i == 'pi_a') {
      for (var j in mydata[i]) {
        mydata[i][j] = Array.from(
          leInt2Buff(unstringifyBigInts(mydata[i][j]), 32)
        ).reverse();
      }
    } else if (i == 'pi_b') {
      for (var j in mydata[i]) {
        for (var z in mydata[i][j]) {
          mydata[i][j][z] = Array.from(
            leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32)
          ); //.reverse();
        }
      }
    } else if (i == 'pi_c') {
      for (var j in mydata[i]) {
        mydata[i][j] = Array.from(
          leInt2Buff(unstringifyBigInts(mydata[i][j]), 32)
        ).reverse();
      }
    }
  }
  let mydataStripped = [
    mydata.pi_a[0],
    mydata.pi_a[1],
    Array.from([].concat.apply([], mydata.pi_b[0])).reverse(),
    Array.from([].concat.apply([], mydata.pi_b[1])).reverse(),
    mydata.pi_c[0],
    mydata.pi_c[1],
  ];
  var merged = [].concat.apply([], mydataStripped);

  return merged;
};
