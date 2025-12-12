use std::path::PathBuf;

pub fn find_light_bin() -> Option<PathBuf> {
    // Run the 'which light' command to find the location of 'light' binary
    #[cfg(not(feature = "devenv"))]
    {
        use std::{fs, process::Command};

        let output = Command::new("which")
            .arg("light")
            .output()
            .expect("Failed to execute 'which light'");

        if !output.status.success() {
            return None;
        }

        let light_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let light_path = PathBuf::from(&light_path);

        // Follow the symlink to find the actual script location.
        // This works for npm, bun, and yarn which all create symlinks from their
        // bin directory to the actual script in the package.
        let symlink_target = fs::read_link(&light_path).ok()?;

        // Resolve relative symlinks (e.g., "../lib/node_modules/...")
        let resolved_path = if symlink_target.is_relative() {
            let parent = light_path.parent()?;
            parent.join(&symlink_target).canonicalize().ok()?
        } else {
            symlink_target.canonicalize().ok()?
        };

        // Navigate up to find the package root (contains bin/ directory with .so files)
        // The symlink target is typically: .../zk-compression-cli/test_bin/run
        // We need to find: .../zk-compression-cli/bin/
        let mut current = resolved_path.as_path();
        while let Some(parent) = current.parent() {
            let bin_dir = parent.join("bin");
            // Check if this bin/ directory contains .so files (our target)
            if bin_dir.exists() && bin_dir.join("account_compression.so").exists() {
                return Some(bin_dir);
            }
            current = parent;
        }

        None
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

#[cfg(all(not(feature = "devenv"), test))]
mod tests {
    use super::*;

    #[test]
    fn test_find_light_bin() {
        let bin_path = find_light_bin();
        println!("find_light_bin() returned: {:?}", bin_path);

        if let Some(path) = &bin_path {
            println!("Path exists: {}", path.exists());
            println!(
                "account_compression.so exists: {}",
                path.join("account_compression.so").exists()
            );
        }

        // Only assert if light CLI is installed
        if bin_path.is_some() {
            let path = bin_path.unwrap();
            assert!(path.exists(), "bin directory should exist");
            assert!(
                path.join("account_compression.so").exists(),
                "account_compression.so should exist in bin directory"
            );
        }
    }
}
