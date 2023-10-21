"use client";
import { AppShell, Paper, Button } from "@mantine/core";

import Image from "next/image";
import lightIcon from "../../public/light-icon.svg";
import {
  IconBook,
  IconBrain,
  IconPlanet,
  IconChartPie2,
} from "@tabler/icons-react";
import { useMediaQuery } from "@mantine/hooks";

const PAGES = [
  /// Label | Path | Enabled | Icon
  ["My assets", "", true, <IconChartPie2 />],
  ["Learn", "trade", false, <IconBrain />],
  ["Explore PSPs", "explore", false, <IconPlanet />],
  ["Developers", "developers", false, <IconBook />],
] as const;

const LightLogo = ({ router }: { router: any }) => (
  <div style={{ display: "flex", paddingLeft: "12px" }}>
    <div
      onClick={() => {
        console.log("push /");
        router.push("/");
      }}
      style={{
        cursor: "pointer",
        display: "flex-start",
        opacity: 1,
        transition: "opacity 0.3s",
      }}
      onMouseOver={(e) => (e.currentTarget.style.opacity = "0.8")}
      onMouseOut={(e) => (e.currentTarget.style.opacity = "1")}
    >
      <Image src={lightIcon} alt="Light Logo" width={25} />
    </div>
  </div>
);

export const BREAKPOINT = "576px";
export const Navbar = ({ router, path }: { router: any; path: any }) => {
  const isMobile = useMediaQuery(`(max-width: ${BREAKPOINT})`);

  return (
    <AppShell.Navbar p="lg">
      <LightLogo router={router} />
      <Paper pt={"lg"}>
        {PAGES.map((page, index) => (
          <Button
            key={index}
            h="60px"
            mt="xs"
            px="lg"
            size={isMobile ? "sm" : "compact-md"}
            fullWidth
            disabled={!page[2]}
            radius="xl"
            variant={path === `/${page[1]}` ? "secondary-active" : "secondary"}
            justify="flex-start"
            leftSection={page[3]}
            onClick={() => console.log(`/${page[1]}`)}
          >
            {page[0]}
          </Button>
        ))}
      </Paper>
    </AppShell.Navbar>
  );
};
