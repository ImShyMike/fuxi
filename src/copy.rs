use crate::cli::confirm;
use std::{fs, path::Path, process::Command};

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub fn sudo_copy(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = dst.parent() {
        let status = Command::new("sudo")
            .arg("mkdir")
            .arg("-p")
            .arg(parent)
            .status()?;
        if !status.success() {
            return Err(format!("sudo mkdir failed for {}", parent.display()).into());
        }
    }
    let status = Command::new("sudo")
        .arg("cp")
        .arg("-a")
        .arg(src.as_os_str())
        .arg(dst.as_os_str())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "sudo cp failed copying {} to {}",
            src.display(),
            dst.display()
        )
        .into())
    }
}

pub fn copy_file_or_path(
    src: &Path,
    dst: &Path,
    folder_contents: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if src.is_dir() {
        if folder_contents {
            // copy only the contents of `src` into `dst`
            // ensure destination directory exists
            if let Err(e) = fs::create_dir_all(dst) {
                if cfg!(unix) {
                    let prompt = format!(
                        "Failed to create destination directory {}: {}. Retry creating it with sudo?",
                        dst.display(),
                        e
                    );
                    if confirm(&prompt)? {
                        let status = Command::new("sudo")
                            .arg("mkdir")
                            .arg("-p")
                            .arg(dst)
                            .status()?;
                        if !status.success() {
                            return Err(format!("sudo mkdir failed for {}", dst.display()).into());
                        }
                    } else {
                        return Err(e.into());
                    }
                } else {
                    return Err(e.into());
                }
            }

            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let src_entry = entry.path();
                let dst_entry = dst.join(entry.file_name());

                if src_entry.is_dir() {
                    if let Err(e) = copy_dir_recursive(&src_entry, &dst_entry) {
                        if cfg!(unix) {
                            let prompt = format!(
                                "Failed to copy directory {} -> {}: {}. Retry with sudo?",
                                src_entry.display(),
                                dst_entry.display(),
                                e
                            );
                            if confirm(&prompt)? {
                                sudo_copy(&src_entry, &dst_entry)?;
                                continue;
                            }
                        }
                        return Err(Box::new(e));
                    }
                } else if let Err(e) = fs::copy(&src_entry, &dst_entry) {
                    if cfg!(unix) {
                        let prompt = format!(
                            "Failed to copy file {} -> {}: {}. Retry with sudo?",
                            src_entry.display(),
                            dst_entry.display(),
                            e
                        );
                        if confirm(&prompt)? {
                            sudo_copy(&src_entry, &dst_entry)?;
                            continue;
                        }
                    }
                    return Err(e.into());
                }
            }
            Ok(())
        } else {
            // copy directory (create dst and copy contents into it)
            if let Err(e) = copy_dir_recursive(src, dst) {
                // if it failed, offer to retry with sudo on unix
                if cfg!(unix) {
                    let prompt = format!(
                        "Failed to copy directory {} -> {}: {}. Retry with sudo?",
                        src.display(),
                        dst.display(),
                        e
                    );
                    if confirm(&prompt)? {
                        return sudo_copy(src, dst);
                    }
                }
                return Err(Box::new(e));
            }
            Ok(())
        }
    } else {
        // ensure parent exists
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                if cfg!(unix) {
                    let prompt = format!(
                        "Failed to create parent directory {}: {}. Retry creating it with sudo?",
                        parent.display(),
                        e
                    );
                    if confirm(&prompt)? {
                        let status = Command::new("sudo")
                            .arg("mkdir")
                            .arg("-p")
                            .arg(parent)
                            .status()?;
                        if !status.success() {
                            return Err(
                                format!("sudo mkdir failed for {}", parent.display()).into()
                            );
                        }
                    } else {
                        return Err(e.into());
                    }
                } else {
                    return Err(e.into());
                }
            }
        }

        match fs::copy(src, dst) {
            Ok(_) => Ok(()),
            Err(e) => {
                // if it failed, offer to retry with sudo on unix
                if cfg!(unix) {
                    let prompt = format!(
                        "Failed to copy file {} -> {}: {}. Retry with sudo?",
                        src.display(),
                        dst.display(),
                        e
                    );
                    if confirm(&prompt)? {
                        return sudo_copy(src, dst);
                    }
                }
                Err(e.into())
            }
        }
    }
}
