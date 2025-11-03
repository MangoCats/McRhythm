//! System information detection
//!
//! **Purpose:** Detect CPU, OS, and audio device information for reporting.
//!
//! **Traceability:** TUNE-OUT-010

use serde::{Deserialize, Serialize};

/// System information for tuning reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// CPU model/name
    pub cpu: String,

    /// Operating system name and version
    pub os: String,

    /// Audio backend (e.g., "ALSA", "PulseAudio", "CoreAudio", "WASAPI")
    pub audio_backend: String,

    /// Audio device name
    pub audio_device: String,
}

impl SystemInfo {
    /// Detect system information
    ///
    /// Attempts to automatically detect CPU, OS, and audio configuration.
    ///
    /// **Platform support:**
    /// - Linux: Reads /proc/cpuinfo and /etc/os-release
    /// - macOS/Windows: Basic detection via std::env
    ///
    /// **Traceability:** TUNE-OUT-010 (system information in report)
    pub fn detect() -> Self {
        Self {
            cpu: Self::detect_cpu(),
            os: Self::detect_os(),
            audio_backend: Self::detect_audio_backend(),
            audio_device: "default".to_string(), // Will be overridden if specific device used
        }
    }

    /// Set audio device name
    pub fn with_device(mut self, device: String) -> Self {
        self.audio_device = device;
        self
    }

    /// Detect CPU information
    fn detect_cpu() -> String {
        #[cfg(target_os = "linux")]
        {
            // Try to read from /proc/cpuinfo
            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                // Look for "model name" line
                for line in content.lines() {
                    if line.starts_with("model name") {
                        if let Some(cpu_name) = line.split(':').nth(1) {
                            return cpu_name.trim().to_string();
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Try sysctl on macOS
            if let Ok(output) = std::process::Command::new("sysctl")
                .arg("-n")
                .arg("machdep.cpu.brand_string")
                .output()
            {
                if let Ok(cpu_name) = String::from_utf8(output.stdout) {
                    return cpu_name.trim().to_string();
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Try WMIC on Windows
            if let Ok(output) = std::process::Command::new("wmic")
                .args(["cpu", "get", "name"])
                .output()
            {
                if let Ok(cpu_info) = String::from_utf8(output.stdout) {
                    // Skip header line
                    if let Some(cpu_name) = cpu_info.lines().nth(1) {
                        return cpu_name.trim().to_string();
                    }
                }
            }
        }

        // Fallback
        "Unknown CPU".to_string()
    }

    /// Detect operating system information
    fn detect_os() -> String {
        #[cfg(target_os = "linux")]
        {
            // Try to read from /etc/os-release
            if let Ok(content) = fs::read_to_string("/etc/os-release") {
                let mut name = String::new();
                let mut version = String::new();

                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        // Use PRETTY_NAME if available (includes version)
                        if let Some(pretty) = line.split('=').nth(1) {
                            return pretty.trim_matches('"').to_string();
                        }
                    } else if line.starts_with("NAME=") {
                        if let Some(n) = line.split('=').nth(1) {
                            name = n.trim_matches('"').to_string();
                        }
                    } else if line.starts_with("VERSION=") {
                        if let Some(v) = line.split('=').nth(1) {
                            version = v.trim_matches('"').to_string();
                        }
                    }
                }

                if !name.is_empty() {
                    return if !version.is_empty() {
                        format!("{} {}", name, version)
                    } else {
                        name
                    };
                }
            }

            // Fallback: Use uname
            if let Ok(output) = std::process::Command::new("uname").arg("-sr").output() {
                if let Ok(os_info) = String::from_utf8(output.stdout) {
                    return os_info.trim().to_string();
                }
            }

            // Final fallback for Linux
            return "Linux".to_string();
        }

        // Use std::env::consts for other platforms
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
            {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return format!("macOS {}", version.trim());
                }
            }
            return "macOS".to_string();
        }

        #[cfg(target_os = "windows")]
        {
            format!("Windows {}", std::env::consts::OS)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            std::env::consts::OS.to_string()
        }
    }

    /// Detect audio backend
    ///
    /// Determines which audio API is being used by cpal.
    ///
    /// **Returns:**
    /// - Linux: "ALSA" (cpal uses ALSA on Linux)
    /// - macOS: "CoreAudio" (cpal uses CoreAudio on macOS)
    /// - Windows: "WASAPI" (cpal uses WASAPI on Windows)
    fn detect_audio_backend() -> String {
        #[cfg(target_os = "linux")]
        {
            "ALSA".to_string()
        }

        #[cfg(target_os = "macos")]
        {
            "CoreAudio".to_string()
        }

        #[cfg(target_os = "windows")]
        {
            "WASAPI".to_string()
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            "Unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_detect() {
        let info = SystemInfo::detect();

        // Should detect something (not empty strings)
        assert!(!info.cpu.is_empty());
        assert!(!info.os.is_empty());
        assert!(!info.audio_backend.is_empty());
        assert_eq!(info.audio_device, "default");

        println!("Detected system info:");
        println!("  CPU: {}", info.cpu);
        println!("  OS: {}", info.os);
        println!("  Audio backend: {}", info.audio_backend);
    }

    #[test]
    fn test_with_device() {
        let info = SystemInfo::detect().with_device("test_device".to_string());
        assert_eq!(info.audio_device, "test_device");
    }

    #[test]
    fn test_audio_backend_known() {
        let backend = SystemInfo::detect_audio_backend();

        // Should be one of the known backends
        assert!(
            backend == "ALSA"
                || backend == "CoreAudio"
                || backend == "WASAPI"
                || backend == "Unknown"
        );
    }
}
