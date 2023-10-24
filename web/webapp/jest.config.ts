module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  transform: {
    "^.+\\.tsx?$": "ts-jest",
  },
  transformIgnorePatterns: [
    "node_modules/(?!(your-project|@adobe/css-tools)/)", // replace 'your-project' with your project name
  ],
  setupFilesAfterEnv: ["<rootDir>/jest.setup.js"],
  globals: {
    "ts-jest": {
      tsconfig: {
        // allow js in ts-jest
        allowJs: true,
      },
    },
  },
};
