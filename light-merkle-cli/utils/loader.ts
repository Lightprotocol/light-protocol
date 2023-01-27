import { log } from "./logger";

// @ts-nocheck
export class Loader {
    private intervalId: any;
    private message: string;
    private animationFrames: string[] = ["|", "/", "-", "\\"];
    private frameIndex = 0;

    constructor(message: string) {
        this.message = message;
    }

    private animate() {
        process.stdout.clearLine(0);
        process.stdout.cursorTo(0);
        process.stdout.write(`${this.message} ${this.animationFrames[this.frameIndex]}`);
        this.frameIndex = (this.frameIndex + 1) % this.animationFrames.length;
    }

    public start() {
        log(this.message, "info")
        this.intervalId = setInterval(() => this.animate(), 1);
    }

    public update(message: string) {
        log(this.message, "info")
        this.message = message;
    }

    public stop(message: string) {
        clearInterval(this.intervalId);
        log(message, "info")
        process.stdout.write(`${message} ${this.animationFrames[this.frameIndex]}`);
        process.stdout.clearLine(0);
        process.stdout.cursorTo(0);
    }
}
