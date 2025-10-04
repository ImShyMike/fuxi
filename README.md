# fuxi

dotfile organization made easy

> [!CAUTION]
> Please be careful when using this program as it is still not fully tested

---

## What is this?

`fuxi` is a CLI tool to help manage, create, and restore backups of your dotfiles using GitHub

## Why?

Keeping track of dotfiles can be hard sometimes so why not automate _most_ of the process

## How do i use it?

## Requirements

- [git](https://git-scm.com/downloads) (should be configured with your Git provider, ex. Github)

### Installation

```bash
cargo install fuxi-cli
```

If instalation fails on Windows, use the following command:

```bash
cargo install fuxi-cli --target x86_64-pc-windows-gnu
```

### Building from source

```bash
git clone https://github.com/ImShyMike/fuxi.git
cd fuxi
cargo build --release
```

## Usage

### Quickstart guide

#### 1. Initialize with a repository

Start by creating a repository (can be private) and get it's local path.

```bash
fuxi init your_username/repo_name REPO_PATH_HERE
```

#### 2. Create a profile

```bash
fuxi profile create main
```

Profiles can be used to store separate dotfile configs.

#### 3. Add paths

```bash
fuxi path add ~/.wakatime.cfg ~/.zshrc
```

#### 4. Create a backup snapshot

```bash
fuxi backup -m "Update wakatime config" --push
```

#### 5. Save repository state without copying files

```bash
fuxi save -m "Sync configuration"
```

This is can be used after manual tweaks inside the backup repository.

#### 6. Apply a backup

```bash
fuxi apply latest
```

Replace `latest` with a specific backup ID or commit hash as needed. Include `--dryrun` to preview the actions without modifying any files.

### Available commands

| Command                                             | Purpose                                                                                                                                             |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fuxi version`                                      | Print the currently installed CLI version.                                                                                                          |
| `fuxi config [-r]`                                  | Show the config file path (`config.toml` under your OS config directory). Use `-r` for the raw path only.                                           |
| `fuxi init <github-repo> <local-path>`              | Register the remote repository (`username/repo`) and the local folder that will store backups. Creates the folder and initializes Git if needed.    |
| `fuxi profile list`                                 | Display every profile and the paths mapped to it.                                                                                                   |
| `fuxi profile create <name>`                        | Create an empty profile. The first profile created becomes the active one automatically.                                                            |
| `fuxi profile switch <name>`                        | Set the active profile.                                                                                                                             |
| `fuxi profile delete <name>`                        | Remove a profile and its path list from the config.                                                                                                 |
| `fuxi path list`                                    | Show the paths tracked by the currently selected profile.                                                                                           |
| `fuxi path add <path> [...]`                        | Register one or more filesystem paths to track. Directories are copied recursively; files are copied one-to-one.                                    |
| `fuxi path remove <path> [...]`                     | Stop tracking one or more paths.                                                                                                                    |
| `fuxi backup [-m <message>] [--push]`               | Copy tracked paths into the repository under `<profile>/<item>` and optionally push the resulting commit to the configured remote.                  |
| `fuxi save [-m <message>] [--force]`                | Commit pending repository changes and push them upstream. Use `--force` to skip the confirmation prompt.                                            |
| `fuxi list`                                         | Show the Git commit history for the backup repository.                                                                                              |
| `fuxi apply <backup-id\|commit\|latest> [--dryrun]` | Fetch and pull the given backup, then copy the stored files back to their original locations. `--dryrun` prints the actions without making changes. |

## License

This project is licensed under the [AGPLv3](https://github.com/ImShyMike/fuxi/blob/main/LICENSE)
