use chrono::{DateTime, Utc};
use clap::{Command, arg};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FuxiConfig {
    platform: Option<String>,
    selected_profile: Option<String>,
    profiles: Option<HashMap<String, Vec<String>>>,
    last_backup_id: Option<String>,
    backup_repo_path: Option<String>,
    github_repo: Option<String>,
    git_branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupMetadata {
    id: String,
    timestamp: DateTime<Utc>,
    paths: Vec<String>,
    commit_hash: Option<String>,
    description: Option<String>,
}

impl Default for FuxiConfig {
    fn default() -> Self {
        Self {
            platform: env::consts::OS.to_string().into(),
            selected_profile: None,
            profiles: None,
            last_backup_id: None,
            backup_repo_path: None,
            github_repo: None,
            git_branch: "main".to_string(),
        }
    }
}

fn cli() -> Command {
    Command::new("fuxi")
        .about("fuxi CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("login").about("Authenticate the user"))
        .subcommand(Command::new("version").about("Show version information"))
        .subcommand(
            Command::new("config")
                .about("Show configuration path")
                .arg(arg!(-r --raw "Output just the directory path")),
        )
        .subcommand(
            Command::new("init")
                .about("Initialize Git backup repository")
                .arg(arg!(<REPO> "GitHub repository (username/repo-name)"))
                .arg(
                    arg!(<PATH> "Local backup repository path")
                        .value_parser(clap::value_parser!(PathBuf)),
                ),
        )
        .subcommand(
            Command::new("profile")
                .about("Manage profiles")
                .subcommand(Command::new("list").about("List all profiles"))
                .subcommand(
                    Command::new("create")
                        .about("Create a new profile")
                        .arg(arg!(<NAME> "Profile name")),
                )
                .subcommand(
                    Command::new("switch")
                        .about("Switch to a profile")
                        .arg(arg!(<NAME> "Profile name")),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a profile")
                        .arg(arg!(<NAME> "Profile name")),
                ),
        )
        .subcommand(
            Command::new("path")
                .about("Manage paths")
                .subcommand(Command::new("list").about("List all paths"))
                .subcommand(Command::new("add").about("Add path(s)").arg(
                    arg!(<PATH> ... "Paths to add").value_parser(clap::value_parser!(PathBuf)),
                ))
                .subcommand(Command::new("remove").about("Remove path(s)").arg(
                    arg!(<PATH> ... "Paths to remove").value_parser(clap::value_parser!(PathBuf)),
                )),
        )
        .subcommand(
            Command::new("backup")
                .about("Create a backup")
                .arg(arg!(-m --message <MESSAGE> "Backup commit message"))
                .arg(arg!(--push "Push to GitHub after backup")),
        )
        .subcommand(
            Command::new("apply")
                .about("Apply a backup ID")
                .arg(arg!(<ID> "Backup ID or commit hash")),
        )
        .subcommand(
            Command::new("save")
                .about("Save current configuration")
                .arg(arg!(--force "Force save without confirmation")),
        )
        .subcommand(Command::new("list").about("List all backups"))
}

fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = ProcessCommand::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git command failed: {}", error).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn push_to_github(
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

fn pull_from_github(repo_path: &Path, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pulling from GitHub...");
    run_git_command(repo_path, &["pull", "origin", branch])?;
    println!("Successfully pulled from GitHub!");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_file_or_path(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;
    let app_config_dir = config_dir.join("fuxi");

    // Create the config directory if it doesn't exist
    std::fs::create_dir_all(&app_config_dir)?;

    Ok(app_config_dir.join("config.toml"))
}

fn load_config() -> Result<FuxiConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;

    let mut builder = Config::builder();

    // Add config file if it exists
    if config_path.exists() {
        builder = builder.add_source(
            File::from(config_path.clone())
                .format(FileFormat::Toml)
                .required(false),
        );
    }

    let config = builder.build()?;

    // Try to deserialize into our struct, fall back to default if it fails
    match config.try_deserialize::<FuxiConfig>() {
        Ok(fuxi_config) => Ok(fuxi_config),
        Err(_) => {
            // If deserialization fails, return default
            Ok(FuxiConfig::default())
        }
    }
}

fn save_config(config: &FuxiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    let config_str = toml::to_string_pretty(config)?;
    fs::write(config_path, config_str)?;
    Ok(())
}

fn add_paths(new_paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config()?;

    let selected = config
        .selected_profile
        .clone()
        .ok_or("No profile selected")?;
    if selected.is_empty() {
        return Err("Please select a profile before adding paths.".into());
    }

    if config.profiles.is_none() {
        config.profiles = Some(HashMap::new());
    }

    let profiles = config.profiles.as_mut().unwrap();
    let paths_vec = profiles.entry(selected.clone()).or_insert_with(Vec::new);

    for path in new_paths {
        let path_str = path.to_string_lossy().to_string();

        if !paths_vec.contains(&path_str) {
            paths_vec.push(path_str);
            println!("Added: {}", path.display());
        } else {
            println!("Path already exists: {}", path.display());
        }
    }

    save_config(&config)?;
    println!("Configuration updated successfully!");
    Ok(())
}

fn remove_paths(paths_to_remove: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config()?;

    let selected = config
        .selected_profile
        .clone()
        .ok_or("No profile selected")?;
    if selected.is_empty() {
        return Err("Please select a profile before trying to remove paths.".into());
    }

    if config.profiles.is_none() {
        config.profiles = Some(HashMap::new());
    }

    let profiles = config.profiles.as_mut().unwrap();
    let paths_vec = profiles.entry(selected.clone()).or_insert_with(Vec::new);

    for path in paths_to_remove {
        let path_str = path.to_string_lossy().to_string();
        if let Some(pos) = paths_vec.iter().position(|x| x == &path_str) {
            paths_vec.remove(pos);
            println!("Removed: {}", path.display());
        } else {
            println!("Path not found: {}", path.display());
        }
    }

    save_config(&config)?;
    println!("Configuration updated successfully!");
    Ok(())
}

fn list_paths() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let paths = get_selected_profile_paths(&config);

    if paths.is_empty() {
        println!("No paths configured.");
    } else {
        println!("Configured paths:");
        for (i, path) in paths.iter().enumerate() {
            println!("  {}: {}", i + 1, path);
        }
    }
    Ok(())
}

fn update_last_backup_id(backup_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config()?;
    config.last_backup_id = Some(backup_id.to_string());
    save_config(&config)?;
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    print!("{} (y/N): ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn get_selected_profile_paths(config: &FuxiConfig) -> Vec<String> {
    if let Some(selected) = &config.selected_profile {
        if let Some(profiles) = &config.profiles {
            if let Some(paths) = profiles.get(selected) {
                return paths.clone();
            }
        }
    }
    Vec::new()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    let _data_dir = dirs::data_dir().unwrap().join("fuxi");
    let _cache_dir = dirs::cache_dir().unwrap().join("fuxi");

    // Load the full configuration using the config crate
    let mut config = load_config()?;

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("login", _)) => {
            println!("Logging in...");
        }
        Some(("version", _)) => {
            println!("fuxi version {}", env!("CARGO_PKG_VERSION"));
        }
        Some(("config", sub_matches)) => {
            if sub_matches.get_flag("raw") {
                println!("{}", config_path.display());
            } else {
                println!("Configuration file: {:?}", config_path);
            }
        }
        Some(("init", sub_matches)) => {
            let repo = sub_matches
                .get_one::<String>("REPO")
                .map(|s| s.as_str())
                .unwrap_or("");
            let path = sub_matches
                .get_one::<PathBuf>("PATH")
                .map(|p| p.as_path())
                .unwrap_or(Path::new(""));
            if path == Path::new("") {
                return Err("Please provide a valid path for the backup repository.".into());
            } else if repo == "" {
                return Err(
                    "Please provide a valid GitHub repository in the format username/repo-name."
                        .into(),
                );
            }

            if confirm(
                "This will initialize a new Git repository at the specified path. Continue?",
            )? == false
            {
                println!("Initialization cancelled.");
                return Ok(());
            }

            config.backup_repo_path = Some(path.to_string_lossy().to_string());
            config.github_repo = Some(repo.to_string());
            save_config(&config)?;
            println!(
                "Backups will use the {} repository at {}",
                repo,
                path.display()
            );
            if !path.exists() {
                fs::create_dir_all(path)?;
                run_git_command(path, &["init"])?;
            }
        }
        Some(("profile", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", _)) => {
                if let Some(profiles) = &config.profiles {
                    for (name, paths) in profiles {
                        println!("Profile: {}", name);
                        for path in paths {
                            println!("  - {}", path);
                        }
                    }
                } else {
                    println!("No profiles found.");
                }
            }
            Some(("create", profile_matches)) => {
                let name = profile_matches
                    .get_one::<String>("NAME")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if config.profiles.is_none() {
                    config.profiles = Some(HashMap::new());
                }

                if let Some(profiles) = &mut config.profiles {
                    if profiles.contains_key(name) {
                        println!("Profile '{}' already exists.", name);
                    } else {
                        profiles.insert(name.to_string(), Vec::new());
                        save_config(&config)?;
                        println!("Profile '{}' created.", name);
                    }
                }

                if config.profiles.as_ref().unwrap().len() == 1 {
                    config.selected_profile = Some(name.to_string());
                    save_config(&config)?;
                    println!("Profile '{}' is now the selected profile.", name);
                }
            }
            Some(("select", profile_matches)) => {
                let name = profile_matches
                    .get_one::<String>("NAME")
                    .map(|s| s.as_str())
                    .unwrap_or("");

                if config.profiles.is_none() {
                    println!("No profiles available. Please create a profile first.");
                    return Ok(());
                }

                if let Some(profiles) = &config.profiles {
                    if profiles.contains_key(name) {
                        config.selected_profile = Some(name.to_string());

                        save_config(&config)?;
                        println!("Switched to profile '{}'.", name);
                    } else {
                        println!("Profile '{}' does not exist.", name);
                    }
                }
            }
            Some(("delete", profile_matches)) => {
                let name = profile_matches
                    .get_one::<String>("NAME")
                    .map(|s| s.as_str())
                    .unwrap_or("");

                if config.profiles.is_none() {
                    println!("Profile '{}' does not exist.", name);
                    return Ok(());
                }

                if let Some(profiles) = &mut config.profiles {
                    if profiles.remove(name).is_some() {
                        if config.selected_profile.as_deref() == Some(name) {
                            config.selected_profile = None;
                            config.profiles.as_mut().unwrap().remove(name);
                        }
                        save_config(&config)?;
                        println!("Profile '{}' deleted.", name);
                    } else {
                        println!("Profile '{}' does not exist.", name);
                    }
                }
            }
            _ => unreachable!(),
        },
        Some(("path", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", _)) => {
                list_paths()?;
            }
            Some(("add", sub_matches)) => {
                let paths: Vec<PathBuf> = sub_matches
                    .get_many::<PathBuf>("PATH")
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect();

                if config.selected_profile.is_none() {
                    println!("Please select a profile before adding paths.");
                    return Ok(());
                }

                add_paths(&paths)?;
            }
            Some(("remove", sub_matches)) => {
                let paths: Vec<PathBuf> = sub_matches
                    .get_many::<PathBuf>("PATH")
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect();
                remove_paths(&paths)?;
            }
            _ => unreachable!(),
        },
        Some(("backup", _)) => {
            let backup_id = format!("backup_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
            update_last_backup_id(&backup_id)?;

            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);

            if config.github_repo.is_none() {
                return Err("GitHub repository is not set. Please run 'fuxi init' first.".into());
            }

            if config.selected_profile.is_none() {
                return Err(
                    "No profile selected. Please select a profile before backing up.".into(),
                );
            }

            let paths = get_selected_profile_paths(&config);
            if paths.is_empty() {
                return Err("No paths configured for the selected profile.".into());
            }

            for path in paths {
                let src_path = Path::new(&path);
                if !src_path.exists() {
                    println!(
                        "Warning: Source path does not exist: {}",
                        src_path.display()
                    );
                    continue;
                }

                // use just the last path component (file or folder)
                let relative_path: PathBuf = src_path
                    .components()
                    .rev()
                    .find_map(|c| {
                        if let std::path::Component::Normal(os_str) = c {
                            Some(PathBuf::from(os_str))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| PathBuf::from(""));

                let selected_profile = config
                    .selected_profile
                    .as_ref()
                    .expect("Selected profile should be present");

                let dst_path = repo_path.join(selected_profile).join(&relative_path);

                copy_file_or_path(src_path, &dst_path)?;
                println!("Backed up {} to {}", src_path.display(), dst_path.display());
            }

            println!("Backup '{}' created successfully!", backup_id);
            println!("Save the bakcup using the 'fuxi save' command.");
        }
        Some(("apply", sub_matches)) => {
            let id = sub_matches
                .get_one::<String>("ID")
                .map(|s| s.as_str())
                .unwrap_or("");
            update_last_backup_id(id)?;

            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);
            let branch = &config.git_branch;

            let result = pull_from_github(repo_path, branch);
            if let Err(e) = result {
                println!("Error during pull: {}", e);
            } else {
                println!("Configuration updated from git repository.");
            }

            let paths = get_selected_profile_paths(&config);
            if paths.is_empty() {
                return Err("No paths configured for the selected profile.".into());
            }

            for path in paths {
                let dst_path: &Path = Path::new(&path);
                if !dst_path.exists() {
                    println!("Warning: Source path does not exist: {}", dst_path.display());
                    continue;
                }

                // if repo_path.exists() {
                //     fs::remove_dir_all(&repo_path)?;
                // }

                copy_file_or_path(repo_path, &dst_path)?;
                println!("Applied {} to {}", repo_path.display(), dst_path.display());
            }

            println!("Backup '{}' applied successfully!", id);
        }
        Some(("save", sub_matches)) => {
            let force = sub_matches.get_flag("force");
            if !force {
                if confirm("Are you sure you want to save the current configuration state?")?
                    == false
                {
                    println!("Save cancelled.");
                    return Ok(());
                }
            }

            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);
            let branch = &config.git_branch;

            let result = push_to_github(repo_path, branch, None);
            if let Err(e) = result {
                println!("Error during push: {}", e);
            } else {
                println!("Configuration saved successfully!");
            }
        }
        Some(("list", _)) => {
            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);
            let log = run_git_command(repo_path, &["log", "--oneline"])?;
            if log.is_empty() {
                println!("No backups found.");
            } else {
                println!("Backups:");
                for line in log.lines() {
                    println!("  {}", line);
                }
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
