import { log } from "../../../utils/logger";
export const print = async () => {
    try {
    } catch (error) {
        let errorMessage = "Aborted.";
        if (error instanceof Error) {
            errorMessage = error.message;
        }
        log(errorMessage, "error");
    }
};
