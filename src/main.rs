// Copyright 2026 Raymond Auge <rayauge@doublebite.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use semver::Version;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{DocumentMut, value};

#[derive(Parser)]
#[command(name = "verz")]
#[command(about = "A semver management tool similar to npm version", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<VerzCommand>,

    /// New version to set
    #[arg(name = "newversion")]
    newversion: Option<String>,

    /// Do not create a git commit and tag
    #[arg(short = 'n', long = "no-git-tag-version")]
    no_git_tag_version: bool,

    /// Commit message
    #[arg(short = 'm', long = "message")]
    message: Option<String>,
}

#[derive(Subcommand)]
enum VerzCommand {
    /// Increment major version
    Major,
    /// Increment minor version
    Minor,
    /// Increment patch version
    Patch,
    /// Increment premajor version
    Premajor,
    /// Increment preminor version
    Preminor,
    /// Increment prepatch version
    Prepatch,
    /// Increment prerelease version
    Prerelease,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let current_version = get_current_version(None)?;
    println!("v{}", current_version);

    let next_version = if let Some(cmd) = cli.command {
        match cmd {
            VerzCommand::Major => increment_version(&current_version, "major")?,
            VerzCommand::Minor => increment_version(&current_version, "minor")?,
            VerzCommand::Patch => increment_version(&current_version, "patch")?,
            VerzCommand::Premajor => increment_version(&current_version, "premajor")?,
            VerzCommand::Preminor => increment_version(&current_version, "preminor")?,
            VerzCommand::Prepatch => increment_version(&current_version, "prepatch")?,
            VerzCommand::Prerelease => increment_version(&current_version, "prerelease")?,
        }
    } else if let Some(ver_str) = cli.newversion {
        Version::parse(&ver_str).context("Invalid version string")?
    } else {
        return Ok(());
    };

    if next_version == current_version {
        return Ok(());
    }

    update_files(&next_version, None)?;

    if !cli.no_git_tag_version && is_git_repo() {
        git_tag_version(&next_version, cli.message)?;
    }

    println!("v{}", next_version);

    Ok(())
}

fn get_current_version(base_path: Option<&Path>) -> Result<Version> {
    let pkg_json = base_path.map_or(PathBuf::from("package.json"), |p| p.join("package.json"));
    let cargo_toml = base_path.map_or(PathBuf::from("Cargo.toml"), |p| p.join("Cargo.toml"));

    if pkg_json.exists() {
        let content = fs::read_to_string(&pkg_json)?;
        let json: JsonValue = serde_json::from_str(&content)?;
        if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
            return Ok(Version::parse(v)?);
        }
    }

    if cargo_toml.exists() {
        let content = fs::read_to_string(&cargo_toml)?;
        let doc = content.parse::<DocumentMut>()?;
        if let Some(v) = doc
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(Version::parse(v)?);
        }
    }

    Err(anyhow!(
        "Could not find version in package.json or Cargo.toml"
    ))
}

fn increment_version(v: &Version, level: &str) -> Result<Version> {
    let mut next = v.clone();
    match level {
        "major" => {
            next.major += 1;
            next.minor = 0;
            next.patch = 0;
            next.pre = semver::Prerelease::EMPTY;
        }
        "minor" => {
            next.minor += 1;
            next.patch = 0;
            next.pre = semver::Prerelease::EMPTY;
        }
        "patch" => {
            if next.pre.is_empty() {
                next.patch += 1;
            }
            next.pre = semver::Prerelease::EMPTY;
        }
        "premajor" => {
            next.major += 1;
            next.minor = 0;
            next.patch = 0;
            next.pre = semver::Prerelease::new("0").map_err(|e| anyhow!(e))?;
        }
        "preminor" => {
            next.minor += 1;
            next.patch = 0;
            next.pre = semver::Prerelease::new("0").map_err(|e| anyhow!(e))?;
        }
        "prepatch" => {
            next.patch += 1;
            next.pre = semver::Prerelease::new("0").map_err(|e| anyhow!(e))?;
        }
        "prerelease" => {
            if next.pre.is_empty() {
                next.patch += 1;
                next.pre = semver::Prerelease::new("0").map_err(|e| anyhow!(e))?;
            } else {
                let pre_str = next.pre.as_str();
                // Find the last numeric part
                if let Some(pos) = pre_str.rfind(|c: char| !c.is_numeric()) {
                    let (prefix, suffix) = pre_str.split_at(pos + 1);
                    if let Ok(num) = suffix.parse::<u64>() {
                        next.pre = semver::Prerelease::new(&format!("{}{}", prefix, num + 1))
                            .map_err(|e| anyhow!(e))?;
                    } else {
                        // If the suffix is not a number, append .0
                        next.pre = semver::Prerelease::new(&format!("{}.0", pre_str))
                            .map_err(|e| anyhow!(e))?;
                    }
                } else {
                    // It's all numeric
                    if let Ok(num) = pre_str.parse::<u64>() {
                        next.pre = semver::Prerelease::new(&(num + 1).to_string())
                            .map_err(|e| anyhow!(e))?;
                    } else {
                        // Not numeric, append .0
                        next.pre = semver::Prerelease::new(&format!("{}.0", pre_str))
                            .map_err(|e| anyhow!(e))?;
                    }
                }
            }
        }
        _ => return Err(anyhow!("Unsupported increment level")),
    }
    Ok(next)
}

fn update_files(v: &Version, base_path: Option<&Path>) -> Result<()> {
    let version_str = v.to_string();
    let pkg_json = base_path.map_or(PathBuf::from("package.json"), |p| p.join("package.json"));
    let cargo_toml = base_path.map_or(PathBuf::from("Cargo.toml"), |p| p.join("Cargo.toml"));

    if pkg_json.exists() {
        let content = fs::read_to_string(&pkg_json)?;
        let mut json: JsonValue = serde_json::from_str(&content)?;
        json["version"] = JsonValue::String(version_str.clone());
        fs::write(&pkg_json, serde_json::to_string_pretty(&json)? + "\n")?;
    }

    if cargo_toml.exists() {
        let content = fs::read_to_string(&cargo_toml)?;
        let mut doc = content.parse::<DocumentMut>()?;
        if let Some(package) = doc.get_mut("package").and_then(|p| p.as_table_mut()) {
            package.insert("version", value(version_str));
            fs::write(&cargo_toml, doc.to_string())?;
        }
    }

    Ok(())
}

fn is_git_repo() -> bool {
    Path::new(".git").exists()
}

fn git_tag_version(v: &Version, message: Option<String>) -> Result<()> {
    let version_str = format!("v{}", v);
    let commit_message = message.unwrap_or_else(|| version_str.clone());

    let files = [
        "package.json",
        "Cargo.toml",
        "package-lock.json",
        "Cargo.lock",
    ];
    for file in files {
        if Path::new(file).exists() {
            Command::new("git").args(["add", file]).status()?;
        }
    }

    Command::new("git")
        .args(["commit", "-m", &commit_message])
        .status()?;

    Command::new("git")
        .args(["tag", "-a", &version_str, "-m", &version_str])
        .status()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_increment_patch() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "patch")?;
        assert_eq!(next.to_string(), "1.2.4");

        let v_pre = Version::parse("1.2.4-0")?;
        let next_pre = increment_version(&v_pre, "patch")?;
        assert_eq!(next_pre.to_string(), "1.2.4");
        Ok(())
    }

    #[test]
    fn test_increment_premajor() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "premajor")?;
        assert_eq!(next.to_string(), "2.0.0-0");
        Ok(())
    }

    #[test]
    fn test_increment_preminor() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "preminor")?;
        assert_eq!(next.to_string(), "1.3.0-0");
        Ok(())
    }

    #[test]
    fn test_increment_prepatch() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "prepatch")?;
        assert_eq!(next.to_string(), "1.2.4-0");
        Ok(())
    }

    #[test]
    fn test_increment_prerelease() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "prerelease")?;
        assert_eq!(next.to_string(), "1.2.4-0");

        let v2 = Version::parse("1.2.4-0")?;
        let next2 = increment_version(&v2, "prerelease")?;
        assert_eq!(next2.to_string(), "1.2.4-1");

        let v3 = Version::parse("1.2.4-alpha.0")?;
        let next3 = increment_version(&v3, "prerelease")?;
        assert_eq!(next3.to_string(), "1.2.4-alpha.1");

        let v4 = Version::parse("1.2.4-alpha")?;
        let next4 = increment_version(&v4, "prerelease")?;
        assert_eq!(next4.to_string(), "1.2.4-alpha.0");
        Ok(())
    }

    #[test]
    fn test_increment_minor() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "minor")?;
        assert_eq!(next.to_string(), "1.3.0");
        Ok(())
    }

    #[test]
    fn test_increment_major() -> Result<()> {
        let v = Version::parse("1.2.3")?;
        let next = increment_version(&v, "major")?;
        assert_eq!(next.to_string(), "2.0.0");
        Ok(())
    }

    #[test]
    fn test_update_package_json() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("package.json");
        fs::write(&file_path, r#"{"name": "test", "version": "1.0.0"}"#)?;

        let next_v = Version::parse("1.1.0")?;
        update_files(&next_v, Some(dir.path()))?;

        let content = fs::read_to_string(file_path)?;
        let json: JsonValue = serde_json::from_str(&content)?;
        assert_eq!(json["version"], "1.1.0");

        Ok(())
    }

    #[test]
    fn test_update_cargo_toml() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("Cargo.toml");
        fs::write(
            &file_path,
            r#"[package]
name = "test"
version = "1.0.0"
"#,
        )?;

        let next_v = Version::parse("2.0.0")?;
        update_files(&next_v, Some(dir.path()))?;

        let content = fs::read_to_string(file_path)?;
        assert!(content.contains(r#"version = "2.0.0""#));

        Ok(())
    }
}
