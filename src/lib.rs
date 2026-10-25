//! odx for Zed: slash commands that put what odx knows about the project in
//! the assistant: `/odx-dashboard [section…]`, `/odx-status`,
//! `/odx-changelog [rev]`, `/odx-glue` and `/odx-sniff [rev]`.
//!
//! The binary is `odx` on the project's PATH (e.g. an activated virtualenv),
//! else the latest `odx-cli` release from PyPI, downloaded once.

mod commands;

use std::fs;

use zed_extension_api::http_client::{HttpMethod, HttpRequest};
use zed_extension_api::{
    self as zed, Result, SlashCommand, SlashCommandArgumentCompletion, SlashCommandOutput,
    SlashCommandOutputSection, Worktree,
};

struct Odx;

/// The platform as [`commands::wheel_tags`] names it.
fn platform() -> (&'static str, &'static str) {
    let (os, arch) = zed::current_platform();
    let os = match os {
        zed::Os::Mac => "mac",
        zed::Os::Linux => "linux",
        zed::Os::Windows => "windows",
    };
    let arch = match arch {
        zed::Architecture::Aarch64 => "aarch64",
        zed::Architecture::X86 => "x86",
        zed::Architecture::X8664 => "x86_64",
    };
    (os, arch)
}

/// The latest `odx-cli` from PyPI, unpacked in the extension's directory.
fn download() -> Result<String> {
    let response = HttpRequest::builder()
        .method(HttpMethod::Get)
        .url("https://pypi.org/pypi/odx-cli/json")
        .build()?
        .fetch()?;
    let index: zed::serde_json::Value = zed::serde_json::from_slice(&response.body)
        .map_err(|e| format!("PyPI's answer for odx-cli: {e}"))?;
    let version = index["info"]["version"]
        .as_str()
        .ok_or("PyPI's answer for odx-cli has no version")?;
    let (os, arch) = platform();
    let (tags, exe) = commands::wheel_tags(os, arch)
        .ok_or("odx: no build for this platform; install odx-cli and put odx on PATH")?;
    let dir = format!("odx-cli-{version}");
    let binary = format!("{dir}/{}", commands::script_in_wheel(version, exe));
    if !fs::metadata(&binary).is_ok_and(|meta| meta.is_file()) {
        let url = commands::pick_wheel(&index["urls"], tags)
            .ok_or_else(|| format!("odx-cli {version} has no build for {os}-{arch}"))?;
        zed::download_file(&url, &dir, zed::DownloadedFileType::Zip)?;
        zed::make_file_executable(&binary)?;
        // Older downloads are not needed any more.
        for entry in fs::read_dir(".").into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("odx-cli-") && name != dir {
                fs::remove_dir_all(entry.path()).ok();
            }
        }
    }
    Ok(binary)
}

impl zed::Extension for Odx {
    fn new() -> Self {
        Self
    }

    fn complete_slash_command_argument(
        &self,
        command: SlashCommand,
        _args: Vec<String>,
    ) -> Result<Vec<SlashCommandArgumentCompletion>> {
        Ok(if command.name == "odx-dashboard" {
            commands::SECTIONS
                .iter()
                .map(|section| SlashCommandArgumentCompletion {
                    label: (*section).to_owned(),
                    new_text: (*section).to_owned(),
                    run_command: true,
                })
                .collect()
        } else {
            Vec::new()
        })
    }

    fn run_slash_command(
        &self,
        command: SlashCommand,
        args: Vec<String>,
        worktree: Option<&Worktree>,
    ) -> Result<SlashCommandOutput> {
        let worktree = worktree.ok_or("odx needs a project")?;
        let arguments = commands::invocation(&command.name, &args, &worktree.root_path())?;
        let program = match worktree.which("odx") {
            Some(path) => path,
            None => download()?,
        };
        let output = zed::process::Command::new(program)
            .args(arguments)
            .envs(worktree.shell_env())
            .output()?;
        let text = commands::text(output.status, &output.stdout, &output.stderr)?;
        Ok(SlashCommandOutput {
            sections: vec![SlashCommandOutputSection {
                range: (0..text.len()).into(),
                label: commands::label(&command.name, &args),
            }],
            text,
        })
    }
}

zed::register_extension!(Odx);
