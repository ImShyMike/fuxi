use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_git_command(
    repo_path: &Path,
    args: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(std::env::vars())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Git command failed with exit code {}:\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn push_to_github(
    repo_path: &Path,
    branch: &str,
    message: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pushing to GitHub...");
    run_git_command(repo_path, &["add", "."])?;

    let status = run_git_command(repo_path, &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        println!("No changes to commit.");
        return Ok(());
    }

    let commit_msg = message.unwrap_or_else(|| "Automated backup commit".to_string());
    run_git_command(repo_path, &["commit", "-m", commit_msg.as_str()])?;
    run_git_command(repo_path, &["push", "origin", branch])?;

    println!("Successfully pushed to GitHub!");
    Ok(())
}

pub fn fetch_from_github(
    repo_path: &Path,
    branch: &str,
    commit_hash: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching from GitHub...");
    // fetch the commit hash if provided, else fetch the branch
    if let Some(hash) = commit_hash {
        run_git_command(repo_path, &["fetch", "origin", hash])?;
        run_git_command(repo_path, &["checkout", hash])?;
    } else {
        run_git_command(repo_path, &["fetch", "origin", branch])?;
        run_git_command(repo_path, &["checkout", branch])?;
        run_git_command(
            repo_path,
            &["reset", "--hard", &format!("origin/{}", branch)],
        )?;
    }
    println!("Successfully fetched from GitHub!");
    Ok(())
}

pub fn pull_from_github(repo_path: &Path, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pulling from GitHub...");
    run_git_command(repo_path, &["pull", "origin", branch])?;
    println!("Successfully pulled from GitHub!");
    Ok(())
}
