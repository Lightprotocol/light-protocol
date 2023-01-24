const chalk = require("chalk");

const TYPES = {
  info: "info",
  error: "error",
  success: "success",
  warning: "warning",
  normal: "normal",
};

const COLORS = {
  [TYPES.info]: {
    text: chalk.blue,
    background: chalk.black.bgBlue,
  },
  [TYPES.success]: {
    text: chalk.green,
    background: chalk.black.bgGreen,
  },
  [TYPES.error]: {
    text: chalk.red,
    background: chalk.black.bgRed,
  },
  [TYPES.warning]: {
    text: chalk.yellow,
    background: chalk.black.bgYellow,
  },
  [TYPES.normal]: {
    text: chalk.white,
    background: chalk.black.white,
  },
};

export const log = (message = "", type = TYPES.info) => {
  const colorType = COLORS[type];
  console.log(`${colorType.text(message)}\n`);
};
