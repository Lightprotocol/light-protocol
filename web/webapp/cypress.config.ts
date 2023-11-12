import { defineConfig } from "cypress";

export default defineConfig({
  e2e: {
    // baseUrl: "http://192.168.1.80:3000",
    setupNodeEvents(on, config) {
      // implement node event listeners here
      on("before:browser:launch", (browser, launchOptions) => {
        if (browser.family === "firefox") {
          // launchOptions.preferences is a map of preference names to values
          // login is not working in firefox when testing_localhost_is_secure_when_hijacked is false
          launchOptions.preferences[
            "network.proxy.testing_localhost_is_secure_when_hijacked"
          ] = true;
        }

        return launchOptions;
      });
    },
  },
});
