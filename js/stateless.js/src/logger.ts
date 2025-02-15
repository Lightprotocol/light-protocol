const winston = require('winston');

// use only if winston is available in your environment, and if you know what
// you are doing.
//
// call logger.info('text', { text: 'test' })
export const logger = winston.createLogger({
    level: 'info',
    format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.prettyPrint(),
    ),
    transports: [
        new winston.transports.File({ filename: 'combined.log' }),
        new winston.transports.Console({
            format: winston.format.prettyPrint(),
        }),
    ],
});
