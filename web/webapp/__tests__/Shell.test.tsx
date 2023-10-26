import { expect, test, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import Shell from "../app/page";
import { Connection } from "@solana/web3.js";

test("Shell component", () => {
  vi.mock("next/navigation", () => ({
    useRouter: () => ({
      route: "/",
      pathname: "/",
      query: "",
      asPath: "/",
    }),
    usePathname: () => "/",
  }));
  vi.mock("@solana/wallet-adapter-react", () => ({
    useConnection: () => ({
      connection: new Connection("https://api.devnet.solana.com"),
    }),
  }));

  render(<Shell />);

  expect(screen.getByRole("navigation")).toBeDefined();
  expect(screen.getByRole("region", { name: /assets/i })).toBeDefined();
  expect(screen.getByRole("region", { name: /transactions/i })).toBeDefined();
});
