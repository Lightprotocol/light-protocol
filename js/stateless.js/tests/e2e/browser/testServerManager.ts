import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

class TestServerManager {
    private serverProcess: ChildProcess | null = null;
    private logStream: fs.WriteStream;

    constructor() {
        const logFilePath = path.resolve(
            __dirname,
            '../../../test-ledger/http-server.log',
        );
        // Create a writable stream for the log file
        this.logStream = fs.createWriteStream(logFilePath, { flags: 'a' });
    }
    startServer(
        port: number = 6042,
        rootDir: string = path.resolve(__dirname, './'),
    ): Promise<void> {
        return new Promise((resolve, reject) => {
            console.log(`Starting server from directory: ${rootDir}`);
            const serverProcess = spawn(
                'http-server',
                [rootDir, '-p', `${port}`], // '-c-1' to disable caching
                {
                    stdio: ['ignore', this.logStream, this.logStream], // Redirect stdout and stderr to the log file
                },
            );

            serverProcess.on('close', code => {
                console.log(`Test server stopped with code ${code}`);
                this.logStream.close(); // Close the log stream when the server stops
            });

            serverProcess.on('error', error => {
                console.error(`Failed to start test server: ${error}`);
                reject(error);
            });

            // Assuming the server is ready immediately without a specific log message to wait for
            this.serverProcess = serverProcess;
            resolve();
        });
    }

    stopServer(): Promise<void> {
        return new Promise((resolve, reject) => {
            if (!this.serverProcess) {
                console.warn('Test server process is not running.');
                return resolve();
            }

            console.log('Stopping test server...');
            this.serverProcess.kill('SIGTERM');
        });
    }
}

export const testServerManager = new TestServerManager();
