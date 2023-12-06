"use client"
import React, { useEffect, useState } from "react";
import { Button, Divider, Group, Paper, Stack, Table, Title } from "@mantine/core";
import { ADMIN_AUTH_KEYPAIR, useWallet } from "@lightprotocol/zk.js";
import { useUser } from "../../state/hooks/useUser";
import { useConnection } from "@solana/wallet-adapter-react";
import { Transactions } from "../../components/Transactions";
import { Assets } from "../../components/Assets";

export default function Vote() {
  console.log("Vote component rendered");

  const wallet = useWallet(
    ADMIN_AUTH_KEYPAIR,
    "https://api.devnet.solana.com",
    false
  );

  const { user, initUser, isLoading, error } = useUser();
  const { connection } = useConnection();

  const [balance, setBalance] = useState(0);
  // TODO: replace with login action.
  // wallet must be available state, but kept separate from login
  useEffect(() => {
    console.log("user", user?.account.getPublicKey());
    console.log("isLoading", isLoading);
    console.log("error", error);
    console.log("walletpubkey", wallet.publicKey.toBase58());
    if (wallet && !user && !isLoading && !error) {
      initUser({ connection, wallet });
    }
    (async () => {
      let balance = await connection.getBalance(wallet.publicKey);
      setBalance(balance);
    })();
  }, []);

  if (isLoading) {
    return <div>Logging in...</div>;
  }

  if (!user) {
    return <div>Please log in</div>;
  }

  const voters: any = [];
  const proposal : any = false;
  const vote : any = false;
  const totalVote : any = 0; 
  return (

      <Stack align="center">
        Public Balance: {balance}
      
        <Stack>
        <Button
        radius={"xl"}
        onClick={() => {
          console.log("creating dao, alice bob charlie, init with values, populate voters state, log sigs");
        }}
      >
        Create Mock Dao
      </Button>
      <Paper radius="md" withBorder w="400px" role="region">
      <Paper style={{ width: "100%", padding: "20px" }}>
        <Title size="xs">Confidential Dao</Title>
      </Paper>
      <Divider></Divider>
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th style={{ width: "30%", padding: "20px" }}>Name</Table.Th>
            <Table.Th style={{ width: "30%", padding: "20px" }}>
              Tokens
            </Table.Th>
            <Table.Th style={{ width: "40%", padding: "20px" }}>
              Vote weight
            </Table.Th>
            <Table.Th style={{ padding: "20px" }}></Table.Th>
          </Table.Tr>
        </Table.Thead>
        {/* {voters.map((voter:any)=> {return (<VoterStakeRow voter={voter}/>)}} */}
      </Table>
    </Paper>
      <Button
        radius={"xl"}
        onClick={() => {
          console.log("creating proposal (log sig)");
        }}
      >
        Create Proposal
      </Button>

        </Stack>
      {proposal && "Test Proposal created"}

      <Group>

        <Button
        radius={"xl"}
        onClick={() => {
          console.log("starting vote (log sig)");
        }}
        >
        Init Vote
      </Button>
      <Button
        radius={"xl"}
        onClick={() => {
          console.log("ending vote");
        }}
        >
        End Vote
      </Button>
        </Group>
        <Stack>

      Voting is: {vote.status === "live" ? "live": "not live"}
      {vote.status === "live" && `Vote count: ${totalVote}`}
      {vote.status === "live" && `Vote result: ${totalVote.value}`}

        </Stack>
      <Button
        radius={"xl"}
        onClick={() => {
          console.log("decrypting vote and publishing (update state, log sig)");
        }}
      >
        Decrypt & Reveal Result
      </Button>
      {vote.status === "done" && `Sig: ${totalVote.sig} Result: ${totalVote.value} `}: 

      </Stack>
  );
}


const VoterStakeRow = ({voter} : {voter:any}) =>{
  return (
    <Stack>
      <Table.Tbody>  
            <Table.Td style={{ padding: "20px" }}>
            {voter.name}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            {voter.tokens}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            {voter.weight}
          </Table.Td>
      </Table.Tbody>
    </Stack>
  )
}

const VoterChoiceRow = ({voter}: {voter:any}) => {
  return (
    // name
    <Stack>
      <Group>
        {voter.name}
        <Button
        radius={"xl"}
        onClick={() => {
          console.log(`${voter.name} vote yes`);
        }}
      >
        Vote Yes
      </Button>
      <Button
        radius={"xl"}
        onClick={() => {
          console.log(`${voter.name} vote no`);
        }}
      >
        Vote No
      </Button>
      </Group>
    </Stack>
  )
}