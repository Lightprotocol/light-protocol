// @ts-nocheck
import { executeWithInput, KEYS } from "../utils/cmd";


describe("The Light CLI", () => {
  describe("Light Compile", () => {
    it("should work on help command", async () => {
      const response = await executeWithInput("Light --help");
      const numberOfMatches = ["Light", "Usage", "init", "help"].filter(
        (x) => response.indexOf(x) > -1
      ).length;
      expect(numberOfMatches).toBeGreaterThan(0);
    });
  });
});
