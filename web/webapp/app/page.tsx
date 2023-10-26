"use client";
import { useDisclosure } from "@mantine/hooks";
import { AppShell, Burger, Group, Stack, Title } from "@mantine/core";
import { usePathname, useRouter } from "next/navigation";
import { Navbar } from "../components/Navbar";
import { ShieldSendModal } from "../components/Modal";
import { useEffect, useState } from "react";
import { ADMIN_AUTH_KEYPAIR, useWallet } from "@lightprotocol/zk.js";
import { useUser } from "../state/hooks/useUser";
import { useConnection } from "@solana/wallet-adapter-react";
import { Transactions } from "../components/Transactions";
import { Assets } from "../components/Assets";

export default function Shell() {
  console.log("Shell component rendered");

  const wallet = useWallet(
    ADMIN_AUTH_KEYPAIR,
    "https://api.devnet.solana.com",
    false
  );

  const [opened, { toggle }] = useDisclosure();
  const router = useRouter();
  const path = usePathname();
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
    return <div>Loggin in...</div>;
  }

  if (!user) {
    return <div>Please log in</div>;
  }

  return (
    <AppShell
      layout="alt"
      header={{ height: 60 }}
      navbar={{ width: 250, breakpoint: "sm", collapsed: { mobile: !opened } }}
      padding="md"
    >
      <Navbar router={router} path={path} />
      <AppShell.Header>
        <Group justify="space-between" h="100%" px="md" pl={"lg"}>
          <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
          <Title size={"sm"}> My assets</Title>
          <Group>
            <ShieldSendModal />
            zk account wallet connection
          </Group>
        </Group>
      </AppShell.Header>
      {/* <AppShell.Main bg={theme.colors?.blue![1]}> */}
      <AppShell.Main bg={"#f3f6f9"}>
        <Stack align="center">
          Public Balance: {balance}
          <Assets />
          <Transactions />
        </Stack>
      </AppShell.Main>
    </AppShell>
  );
}
