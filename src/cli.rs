use std::path::PathBuf;

use clap::{Command, arg};

pub fn confirm(prompt: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    print!("{} (y/N): ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

pub fn cli() -> Command {
    Command::new("fuxi")
        .about("fuxi CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
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
                .arg(arg!(<ID> "Backup ID or commit hash"))
                .arg(arg!(-d --dryrun "Show what would be done without making changes")),
        )
        .subcommand(
            Command::new("save")
                .about("Save current configuration")
                .arg(arg!(-m --message <MESSAGE> "Commit message"))
                .arg(arg!(--force "Force save without confirmation")),
        )
        .subcommand(Command::new("list").about("List all backups"))
}
