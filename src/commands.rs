//! What each slash command runs, and how its output is labelled. No Zed API
//! here (only its `serde_json`), so it is tested on the host.

use zed_extension_api::serde_json;

/// The dashboard sections `/odx-dashboard` completes.
pub const SECTIONS: [&str; 9] = [
    "pinned",
    "droppable",
    "new-upstream",
    "forward-port",
    "backport",
    "auto-update",
    "glue",
    "drift",
    "suppressed",
];

/// The arguments of `odx` for the slash command `name` with `args`, in the
/// project at `root`.
pub fn invocation(name: &str, args: &[String], root: &str) -> Result<Vec<String>, String> {
    let mut out = vec!["-C".to_owned(), root.to_owned()];
    match name {
        "odx-dashboard" => {
            out.extend(["dashboard", "--format", "markdown"].map(str::to_owned));
            for section in args {
                if !SECTIONS.contains(&section.as_str()) {
                    return Err(format!(
                        "unknown section {section}: expected one of {}",
                        SECTIONS.join(", ")
                    ));
                }
                out.extend(["--section".to_owned(), section.clone()]);
            }
        }
        "odx-status" => out.push("status".to_owned()),
        "odx-glue" => out.push("glue".to_owned()),
        "odx-changelog" | "odx-sniff" => {
            out.push(name.trim_start_matches("odx-").to_owned());
            out.extend(["--since".to_owned(), since(args)?]);
        }
        _ => return Err(format!("unknown slash command {name}")),
    }
    Ok(out)
}

/// The revision of `/odx-changelog` and `/odx-sniff`: one, or `HEAD`.
fn since(args: &[String]) -> Result<String, String> {
    match args {
        [] => Ok("HEAD".to_owned()),
        [rev] => Ok(rev.clone()),
        _ => Err("expected one revision".to_owned()),
    }
}

/// The label of the output of `name` with `args`.
pub fn label(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("odx {}", name.trim_start_matches("odx-"))
    } else {
        format!("odx {} {}", name.trim_start_matches("odx-"), args.join(" "))
    }
}

/// The text a run produces: its output, or what went wrong.
pub fn text(status: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(stdout).into_owned();
    if status == Some(0) {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(stderr);
    let reason = stderr.trim().lines().last().unwrap_or("").to_owned();
    Err(if reason.is_empty() {
        format!(
            "odx exited with {}",
            status.map_or_else(|| "a signal".to_owned(), |code| format!("code {code}"))
        )
    } else {
        reason
    })
}

/// The fragments of the wheel file name for a platform, as PyPI names
/// them, and the name of the executable.
pub fn wheel_tags(os: &str, arch: &str) -> Option<(&'static [&'static str], &'static str)> {
    Some(match (os, arch) {
        ("linux", "x86_64") => (&["manylinux", "x86_64"], "odx"),
        ("linux", "aarch64") => (&["manylinux", "aarch64"], "odx"),
        ("mac", "aarch64") => (&["macosx", "arm64"], "odx"),
        ("mac", "x86_64") => (&["macosx", "x86_64"], "odx"),
        ("windows", "x86_64") => (&["win_amd64"], "odx.exe"),
        _ => return None,
    })
}

/// The URL of the wheel among PyPI's `urls` of a release whose name has
/// every one of `tags`.
pub fn pick_wheel(urls: &serde_json::Value, tags: &[&str]) -> Option<String> {
    urls.as_array()?.iter().find_map(|file| {
        let name = file.get("filename")?.as_str()?;
        (std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("whl"))
            && tags.iter().all(|tag| name.contains(tag)))
        .then(|| file.get("url")?.as_str().map(str::to_owned))
        .flatten()
    })
}

/// Where the executable is inside the unpacked wheel of `version`.
pub fn script_in_wheel(version: &str, exe: &str) -> String {
    format!("odx_cli-{version}.data/scripts/{exe}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn dashboard_sections_are_checked() {
        assert_eq!(
            invocation("odx-dashboard", &strings(&["glue", "drift"]), "/p").unwrap(),
            strings(&[
                "-C",
                "/p",
                "dashboard",
                "--format",
                "markdown",
                "--section",
                "glue",
                "--section",
                "drift"
            ])
        );
        assert!(
            invocation("odx-dashboard", &strings(&["later"]), "/p")
                .unwrap_err()
                .starts_with("unknown section later: expected one of pinned,")
        );
    }

    #[test]
    fn revisions_default_to_head() {
        assert_eq!(
            invocation("odx-changelog", &[], "/p").unwrap(),
            strings(&["-C", "/p", "changelog", "--since", "HEAD"])
        );
        assert_eq!(
            invocation("odx-sniff", &strings(&["v1.4"]), "/p").unwrap()[2..],
            strings(&["sniff", "--since", "v1.4"])
        );
        assert_eq!(
            invocation("odx-sniff", &strings(&["a", "b"]), "/p").unwrap_err(),
            "expected one revision"
        );
    }

    #[test]
    fn other_commands_take_no_arguments() {
        assert_eq!(
            invocation("odx-status", &[], "/p").unwrap()[2..],
            strings(&["status"])
        );
        assert_eq!(
            invocation("odx-glue", &[], "/p").unwrap()[2..],
            strings(&["glue"])
        );
        assert_eq!(
            invocation("odx-other", &[], "/p").unwrap_err(),
            "unknown slash command odx-other"
        );
    }

    #[test]
    fn labels_name_the_command() {
        assert_eq!(label("odx-status", &[]), "odx status");
        assert_eq!(
            label("odx-changelog", &strings(&["v1.4"])),
            "odx changelog v1.4"
        );
    }

    #[test]
    fn failures_say_why() {
        assert_eq!(text(Some(0), b"ok\n", b"").unwrap(), "ok\n");
        assert_eq!(
            text(Some(2), b"", b"warning: x\nerror: no pyproject.toml\n").unwrap_err(),
            "error: no pyproject.toml"
        );
        assert_eq!(
            text(Some(1), b"", b"").unwrap_err(),
            "odx exited with code 1"
        );
        assert_eq!(
            text(None, b"", b"").unwrap_err(),
            "odx exited with a signal"
        );
    }

    #[test]
    fn wheels_per_platform() {
        let urls = serde_json::json!([
            {"filename": "odx_cli-0.0.7-py3-none-macosx_11_0_arm64.whl", "url": "https://f/mac-arm"},
            {"filename": "odx_cli-0.0.7-py3-none-manylinux_2_17_x86_64.manylinux2014_x86_64.whl", "url": "https://f/linux"},
            {"filename": "odx_cli-0.0.7-py3-none-musllinux_1_2_x86_64.whl", "url": "https://f/musl"},
            {"filename": "odx_cli-0.0.7-py3-none-win_amd64.whl", "url": "https://f/win"},
            {"filename": "odx_cli-0.0.7.tar.gz", "url": "https://f/sdist"}
        ]);
        let url = |os, arch| wheel_tags(os, arch).and_then(|(tags, _)| pick_wheel(&urls, tags));
        assert_eq!(url("linux", "x86_64").as_deref(), Some("https://f/linux"));
        assert_eq!(url("mac", "aarch64").as_deref(), Some("https://f/mac-arm"));
        assert_eq!(url("windows", "x86_64").as_deref(), Some("https://f/win"));
        assert_eq!(url("mac", "x86_64"), None);
        assert_eq!(wheel_tags("windows", "aarch64"), None);
        assert_eq!(pick_wheel(&serde_json::json!({}), &["x"]), None);
        assert_eq!(
            script_in_wheel("0.0.7", "odx.exe"),
            "odx_cli-0.0.7.data/scripts/odx.exe"
        );
    }
}
