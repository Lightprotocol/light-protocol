import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export async function freePort(port: number = 6042): Promise<void> {
    const findProcessCmd =
        process.platform === 'win32'
            ? `netstat -ano | findstr :${port}`
            : `lsof -i :${port} | grep LISTEN | awk '{print $2}'`;

    try {
        const { stdout, stderr } = await execAsync(findProcessCmd);
        if (!stdout.trim()) {
            console.log(`Port ${port} is free.`);
            return;
        }

        // Extract PID(s) and kill
        const pids = stdout.match(/\d+/g);
        if (pids) {
            pids.forEach(pid => {
                console.log(`Killing process ${pid} on port ${port}`);
                process.kill(Number(pid), 'SIGKILL');
            });
        }
        console.log(`Port ${port} is now free.`);
    } catch (err) {
        console.error(`Error checking or freeing port ${port}:`, err);
    }
}
