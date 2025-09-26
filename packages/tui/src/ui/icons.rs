/// Download status icons - clean and immediately recognizable
pub struct DownloadIcons;

impl DownloadIcons {
    pub const PENDING: &'static str = "󰄯"; // nf-mdi-pause
    pub const CONNECTING: &'static str = "󰑐"; // nf-mdi-dots_horizontal
    pub const DOWNLOADING: &'static str = "󰉍"; // nf-mdi-download
    pub const COMPLETED: &'static str = "󰗠"; // nf-mdi-check_circle
    pub const FAILED: &'static str = "󰂭"; // nf-mdi-close_circle
}

/// File type icons - semantic and clear
pub struct FileIcons;

impl FileIcons {
    pub const MODEL: &'static str = "󰧮"; // nf-mdi-brain
    pub const CONFIG: &'static str = "󰒓"; // nf-mdi-cog
    pub const FILE: &'static str = "󰈙"; // nf-mdi-file
    pub const README: &'static str = "󰈙"; // nf-mdi-file_document
}

/// Network and performance icons
pub struct NetworkIcons;

impl NetworkIcons {
    pub const NETWORK: &'static str = "󰛳"; // nf-mdi-web
    pub const BANDWIDTH: &'static str = "󰓅"; // nf-mdi-speedometer
}

/// UI chrome and navigation icons
pub struct UIIcons;

impl UIIcons {
    pub const SELECTED: &'static str = "▶"; // Right triangle
    pub const INFO: &'static str = "󰋽"; // nf-mdi-information
}

/// Model and AI specific icons
pub struct ModelIcons;

impl ModelIcons {
    pub const HUGGINGFACE: &'static str = "󰤃"; // nf-mdi-face_man
}

/// System icons
pub struct SystemIcons;

impl SystemIcons {
    pub const DISK: &'static str = "󰋊"; // nf-mdi-harddisk
}

/// Progress icons
pub struct ProgressIcons;

impl ProgressIcons {
    pub const CLOCK: &'static str = "󰥔"; // nf-mdi-clock_outline
}

/// Icon utilities
pub struct IconUtils;

impl IconUtils {
    pub fn download_status_icon(status: &crate::ui::state::DownloadStatus) -> &'static str {
        use crate::ui::state::DownloadStatus;
        match status {
            DownloadStatus::Pending => DownloadIcons::PENDING,
            DownloadStatus::Connecting => DownloadIcons::CONNECTING,
            DownloadStatus::Downloading => DownloadIcons::DOWNLOADING,
            DownloadStatus::Completed => DownloadIcons::COMPLETED,
            DownloadStatus::Failed => DownloadIcons::FAILED,
        }
    }

    pub fn file_type_icon(filename: &str) -> &'static str {
        match filename.split('.').next_back().unwrap_or("") {
            "safetensors" | "bin" | "pt" | "pth" | "ckpt" => FileIcons::MODEL,
            "json" | "yaml" | "yml" | "toml" | "cfg" => FileIcons::CONFIG,
            "md" | "txt" | "rst" => FileIcons::README,
            _ => FileIcons::FILE,
        }
    }
}
