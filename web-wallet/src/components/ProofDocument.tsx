import {
  Document,
  Page,
  View,
  Link as PDFLink,
  Text,
} from "@react-pdf/renderer";
import { DECIMALS, DECIMALS_SOL, Token } from "../constants";

export const ProofDocument = ({
  data,
  tx,
  publicKey = "DATA DID NOT LOAD CORRECTLY, PLEASE RELOAD AND CREATE A FRESH PDF",
  activeToken,
}) => {
  return (
    // @ts-ignore

    <Document>
      {/* @ts-ignore */}
      <Page>
        <View style={{ display: "flex", justifyContent: "space-between" }}>
          <View
            style={{
              fontSize: "14px",
              fontWeight: "bold",
              paddingLeft: "60px",
              marginTop: "32px",
            }}
          >
            <Text>Source of Funds Proof for Wallet</Text>
          </View>
          <View
            style={{
              fontSize: "14px",
              fontWeight: "bold",
              paddingLeft: "60px",
              marginTop: "4px",
            }}
          >
            <Text>
              <b>{tx.to}</b>
            </Text>
          </View>
        </View>
        <View
          style={{
            fontSize: "10px",
            marginVertical: "16px",
          }}
        >
          <View
            style={{
              fontSize: "10px",
              paddingHorizontal: "60px",
              marginBottom: "24px",
            }}
          >
            <Text>
              The aforementioned Solana wallet was funded through an off-ramp
              from the Light Protocol ZK layer in the following transaction:
            </Text>
          </View>
          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
              marginBottom: "4px",
              paddingRight: "80px",
            }}
          >
            <Text>
              Amount:{" "}
              {tx?.amount && activeToken === Token.SOL
                ? tx.amount / DECIMALS_SOL
                : tx?.amount && activeToken === Token.USDC
                ? tx.amount / DECIMALS
                : "--"}{" "}
              {activeToken} -- Transaction signature: {tx.signature}
            </Text>
          </View>
          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
              marginBottom: "24px",
            }}
          >
            <View
              style={{
                fontSize: "8px",
                marginTop: "2px",
                paddingHorizontal: "20px",
              }}
            >
              <PDFLink src={`https://explorer.solana.com/tx/${tx.signature}`}>
                {"Transaction explorer link"}
              </PDFLink>
            </View>
          </View>
          <View
            style={{
              fontSize: "10px",
              paddingHorizontal: "60px",
              marginBottom: "24px",
            }}
          >
            <Text>
              The user's Light Protocol balance was originally funded by{" "}
              {
                data.filter((d) => d.transaction.type === "shield")[0]
                  ?.transaction.signer
              }
              , which is linked to the user's wallet: {publicKey}, via the
              following on-ramp transaction(s):
            </Text>
          </View>

          {data
            .filter((d) => d.transaction.type === "shield")
            .map((trx, index) => (
              <View
                style={{
                  // @ts-ignore
                  key: index,
                  fontSize: "8px",
                  paddingHorizontal: "60px",
                  marginBottom: "24px",
                }}
              >
                <Text>Transaction signature: {trx?.transaction.signature}</Text>
                <View
                  style={{
                    fontSize: "8px",
                    marginTop: "2px",
                    paddingHorizontal: "20px",
                  }}
                >
                  <Text>
                    {trx?.outUtxo?.commitment
                      ? "commitment created: " + trx?.outUtxo?.commitment + " *"
                      : ""}
                  </Text>
                </View>
                {trx?.inUtxo?.nullifierPda && (
                  <View
                    style={{
                      fontSize: "8px",
                      marginTop: "2px",
                      paddingHorizontal: "20px",
                    }}
                  >
                    <Text>
                      {"nullifier spent: " + trx?.inUtxo?.nullifierPda + " **"}
                    </Text>
                  </View>
                )}
                <View
                  style={{
                    fontSize: "8px",
                    marginTop: "2px",
                    paddingHorizontal: "20px",
                  }}
                >
                  <PDFLink
                    src={`https://explorer.solana.com/tx/${trx?.transaction?.signature}`}
                  >
                    {"Transaction explorer link"}
                  </PDFLink>
                </View>
              </View>
            ))}
          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
              marginTop: "16px",
            }}
          >
            <Text>
              * Respective commitment hashes are provably part of the account
              state of one of the off-ramp transaction's account keys:{" "}
              {tx.leaves.pda}
            </Text>
          </View>
          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
              marginTop: "4px",
            }}
          >
            <Text>
              ** Respective nullifiers (if applicable) are provably part of the
              transactions account keys.
            </Text>
          </View>

          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
              marginTop: "4px",
            }}
          >
            <Text>
              To verify the data, refer to the respective transaction signatures
              via a transaction explorer.
            </Text>
          </View>

          <View
            style={{
              fontSize: "8px",
              paddingHorizontal: "60px",
            }}
          >
            <Text>
              If you have questions about how to use this document feel free to
              send an email to: compliance@lightprotocol.com.
            </Text>
          </View>
        </View>
        <View
          style={{
            fontSize: "8px",
            paddingHorizontal: "60px",
            marginTop: "16px",
          }}
        >
          <Text>
            Light Protocol is a ZK infrastructure layer for Solana that allows
            for various use cases to be built on top of it using ZK technology.
            You can find more information at: https://lightprotocol.com
          </Text>
          <Text>
            THIS DOCUMENT HAS BEEN AUTOMATICALLY GENERATED USING THE WEBSITE:
            https://shield.lightprotocol.com OR IT'S CORRESPONDING APIS ON{" "}
            {new Date().toString()}. ONLY THE USER OWNS THEIR OWN DATA. LIGHT
            PROTOCOL LABS BUILDS DECENTRALIZED AND TRUSTLESS ZK INFRASTRUCTURE
            AND DOESNT COLLECT AND STORE ANY DATA.
          </Text>
        </View>
      </Page>
    </Document>
  );
};
