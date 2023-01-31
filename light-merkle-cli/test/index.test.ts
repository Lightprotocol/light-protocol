// @ts-nocheck
import { executeWithInput } from "../utils/cmd";
import { sleep } from "../utils/utils";
jest.setTimeout(10000)

describe("Merkle Tree Config CLI", () => {
  afterAll(async () => {
    await new Promise(resolve => setTimeout(() => resolve(), 5000)); // avoid jest open handle error
  });
  describe("CLI", () => {
    it("should work on help command", async () => {
      const response = await executeWithInput("npm run start help");
      const numberOfMatches = ["configure", "initialize", "authority", "verifier"].filter(
        (x) => response.indexOf(x) > -1
      ).length;
      expect(numberOfMatches).toBe(4);

    });
  });

  describe("Authority", () => {
    it("should create an authority account", async () => {
      const response = await executeWithInput("npm run start authority init");
      expect(response.includes("Merkle Tree Authority PubKey: 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y")).toBe(true);
    });
    it("should throw an error if again authority account is created", async () => {
      const response = await executeWithInput("npm run start authority init");
      expect(response.includes("account Address { address: 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y, base: None } already in use")).toBe(true);
    });

  });

  describe("merkle tree initialize", () => {
    it("should intialize new merkle tree", async () => {
      const response = await executeWithInput("npm run start initialize DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU")
      expect(response.includes("Merkle Tree PubKey: DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU")).toBe(true);
    });

  })

  describe("Verifier Accounts", () => {

    it("should register an verifier: 1", async () => {
      const response = await executeWithInput("npm run start verifier set J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i")
      expect(response.includes("Verifier PubKey: J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i")).toBe(true);

    });

    it("should register an verifier: 2", async () => {
      const response = await executeWithInput("npm run start verifier set 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL")
      expect(response.includes("Verifier PubKey: 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL")).toBe(true);

    });

    it("should register an verifier: 3", async () => {
      const response = await executeWithInput("npm run start verifier set GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8")
      expect(response.includes("Verifier PubKey: GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8")).toBe(true);
    });

    it("should get an verifier", async () => {
      const response = await executeWithInput("npm run start verifier get 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL")
      expect(response.includes("PublicKey [PublicKey(3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL)]")).toBe(true);
    });
  })

  describe("Pool Accounts", () => {

    it("register pool type account", async () => {
      const response = await executeWithInput("npm run start pool pooltype")
      expect(response.includes("Successfully registered pool type")).toBe(true)

    });

    it("should enable the permissionless spl tokens in authority account", async () => {
      const response = await executeWithInput("npm run start pool sol")
      expect(response.includes("Successfully registered sol pool")).toBe(true)

    });
  })

  describe("Configure Accounts", () => {

    it("should enable the nft in authority account", async () => {
      const response = await executeWithInput("npm run start configure nfts")
      expect(response.includes("Nfts tokens enabled")).toBe(true)
    });


    it("should enable the permissionless spl tokens in authority account", async () => {
      const response = await executeWithInput("npm run start configure spl")
      expect(response.includes("Spl tokens enabled")).toBe(true)
    });


    it("should update the lock duration", async () => {
      const response = await executeWithInput("npm run start configure lock 2000")
      expect(response.includes(`Lock Duration updated: 2000`)).toBe(true)
    });

  })

});
