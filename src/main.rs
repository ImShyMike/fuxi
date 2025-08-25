mod cfg;
mod cli;
mod copy;
mod git;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cfg::{FuxiConfig, get_config_path, load_config, save_config};
use cli::{cli, confirm};
use copy::copy_file_or_path;
use git::{fetch_from_github, pull_from_github, push_to_github, run_git_command};

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

    let mut config = load_config()?;

    let matches = cli().get_matches();
    match matches.subcommand() {
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
            } else if repo.is_empty() {
                return Err(
                    "Please provide a valid GitHub repository in the format username/repo-name."
                        .into(),
                );
            }

            if !(confirm(
                "This will initialize a new Git repository at the specified path. Continue?",
            )?) {
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
        Some(("backup", sub_matches)) => {
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

                copy_file_or_path(src_path, &dst_path, false)?;
                println!("Backed up {} to {}", src_path.display(), dst_path.display());
            }

            println!("Backup '{}' created successfully!", backup_id);

            if sub_matches.get_flag("push") {
                let message = sub_matches
                    .get_one::<String>("message")
                    .cloned()
                    .unwrap_or_else(|| format!("Backup {}", backup_id));
                let branch = &config.git_branch;
                let result = push_to_github(repo_path, branch, Some(message));
                if let Err(e) = result {
                    println!("Error during push: {}", e);
                } else {
                    println!("Backup pushed to GitHub successfully!");
                }
            } else {
                println!("Save the backup using the 'fuxi save' command.");
            }
        }
        Some(("apply", sub_matches)) => {
            let id = sub_matches
                .get_one::<String>("ID")
                .map(|s| s.as_str())
                .unwrap_or("");
            update_last_backup_id(id)?;

            if id == "latest" {
                if let Some(last_id) = &config.last_backup_id {
                    println!("Using last backup ID: {}", last_id);
                } else {
                    return Err("No last backup ID found.".into());
                }
            } else {
                // check if id is a valid commit hash or backup ID
                if id.len() < 7 {
                    return Err("Please provide a valid backup ID or commit hash.".into());
                }
            }

            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);
            let branch = &config.git_branch;

            let log = run_git_command(repo_path, &["log", "--oneline"])?;
            if log.is_empty() {
                return Err("No backups found in the repository.".into());
            }

            if id == "latest" {
                // fetch latest from GitHub
                if let Err(e) = fetch_from_github(repo_path, branch, None) {
                    println!("Error during fetch: {}", e);
                    return Ok(());
                } else {
                    println!("Fetched the latest backup from git repository.");
                }
            } else {
                if !log.contains(id) {
                    return Err(format!("Backup ID or commit hash '{}' not found.", id).into());
                }

                if let Err(e) = fetch_from_github(repo_path, branch, Some(id)) {
                    println!("Error during fetch: {}", e);
                    return Ok(());
                } else {
                    println!("Fetched the specified backup from git repository.");
                }
            }

            // pull latest changes
            if let Err(e) = pull_from_github(repo_path, branch) {
                println!("Error during pull: {}", e);
            } else {
                println!("Configuration updated from git repository.");
            }

            let paths = get_selected_profile_paths(&config);
            if paths.is_empty() {
                return Err("No paths configured for the selected profile.".into());
            }

            let selected_profile = config
                .selected_profile
                .as_ref()
                .expect("Selected profile should be present");

            let dry_run = sub_matches.get_flag("dryrun");

            for path in paths {
                let dst_path: &Path = Path::new(&path);
                if !dst_path.exists() {
                    println!(
                        "Warning: Source path does not exist: {}",
                        dst_path.display()
                    );
                    continue;
                }

                let src_path = Path::new(&path);
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

                // if repo_path.exists() {
                //     fs::remove_dir_all(&repo_path)?;
                // }

                let src_path = repo_path.join(selected_profile).join(&relative_path);
                if !src_path.exists() {
                    println!(
                        "Warning: Backup path does not exist in repository: {}",
                        src_path.display()
                    );
                    continue;
                }

                if !dry_run {
                    copy_file_or_path(&src_path, dst_path, true)?;
                    println!("Applied {} to {}", src_path.display(), dst_path.display());
                } else {
                    println!(
                        "[Dry Run] Would apply {} to {}",
                        src_path.display(),
                        dst_path.display()
                    );
                }
            }

            println!("Backup '{}' applied successfully!", id);
        }
        Some(("save", sub_matches)) => {
            let force = sub_matches.get_flag("force");
            if !force
                && !(confirm("Are you sure you want to save the current configuration state?")?)
            {
                println!("Save cancelled.");
                return Ok(());
            }

            let repo_path = config
                .backup_repo_path
                .as_ref()
                .ok_or("Backup repository path is not set. Please run 'fuxi init' first.")?;
            let repo_path = Path::new(repo_path);
            let branch = &config.git_branch;
            let message = sub_matches
                .get_one::<String>("message")
                .cloned()
                .unwrap_or_else(|| "Save configuration".to_string());

            let result = push_to_github(repo_path, branch, Some(message));
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
