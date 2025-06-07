use std::path::PathBuf;

pub fn find_light_bin() -> Option<PathBuf> {
    // Run the 'which light' command to find the location of 'light' binary
    #[cfg(not(feature = "devenv"))]
    {
        use std::process::Command;
        let output = Command::new("which")
            .arg("light")
            .output()
            .expect("Failed to execute 'which light'");

        if !output.status.success() {
            return None;
        }
        // Convert the output into a string (removing any trailing newline)
        let light_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Get the parent directory of the 'light' binary
        let mut light_bin_path = PathBuf::from(light_path);
        light_bin_path.pop(); // Remove the 'light' binary itself

        // Assuming the node_modules path starts from '/lib/node_modules/...'
        let node_modules_bin =
            light_bin_path.join("../lib/node_modules/@lightprotocol/zk-compression-cli/bin");

        Some(node_modules_bin.canonicalize().unwrap_or(node_modules_bin))
    }
    #[cfg(feature = "devenv")]
    {
        println!("Use only in light protocol monorepo. Using 'git rev-parse --show-toplevel' to find the location of 'light' binary");
        let light_protocol_toplevel = String::from_utf8_lossy(
            &std::process::Command::new("git")
                .arg("rev-parse")
                .arg("--show-toplevel")
                .output()
                .expect("Failed to get top-level directory")
                .stdout,
        )
        .trim()
        .to_string();
        let light_path = PathBuf::from(format!("{}/target/deploy/", light_protocol_toplevel));
        Some(light_path)
    }
}
