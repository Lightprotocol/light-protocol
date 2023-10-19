// import { TokenUtxoBalance } from "@lightprotocol/zk.js";

// function parseBalance(tokenBalance: TokenUtxoBalance) {
//   let _token = tokenBalance.tokenData.symbol;
//   let balance =
//     _token === "SOL"
//       ? tokenBalance.totalBalanceSol.toString()
//       : tokenBalance.totalBalanceSpl.toString();
//   let utxoNumber = tokenBalance.utxos.size;

//   return {
//     token: _token,
//     balance: balance,
//     utxos: utxoNumber,
//   };
// }
// export const BalanceBox = ({
//   balance,
// }: {
//   balance: Map<string, TokenUtxoBalance> | undefined;
// }) => {
//   return (
//     <Box className="balances">
//       {balance &&
//         Array.from(balance.keys()).map((token, index) => {
//           const tokenBalance = balance.get(token);
//           return tokenBalance ? (
//             <Box key={index} className="balance-item modern-look">
//               <Text className="token modern-look">{token}</Text>
//               <Text className="amount modern-look">
//                 {parseBalance(tokenBalance).balance}
//               </Text>
//             </Box>
//           ) : null;
//         })}
//     </Box>
//   );
// };
