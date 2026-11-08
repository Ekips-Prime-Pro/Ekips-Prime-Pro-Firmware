//! Host-side helper tasks for the Ekips Prime Pro firmware.
//!
//! Usage (from the repository root):
//!
//! ```text
//! cargo xtask bin [--debug] [--features <list>]   build the firmware and write target/firmware.bin
//! cargo xtask check-elf <path>                    run the bootloader safety checks on an ELF file
//! ```
//!
//! `bin` never flashes anything. It only produces a raw binary for
//! `dfu-util`, after verifying that no loadable byte of the image lies in
//! the LEGO bootloader range `0x08000000..0x08008000`.

mod elf;
mod image;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const TARGET: &str = "thumbv7em-none-eabihf";
const BIN_NAME: &str = "ekips-prime-pro-firmware";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("bin") => cmd_bin(&args[1..]),
        Some("check-elf") => match args.get(1) {
            Some(path) => check_elf(Path::new(path)).map(|_| ()),
            None => Err("usage: cargo xtask check-elf <path>".into()),
        },
        _ => Err(usage()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> String {
    "usage: cargo xtask <bin [--debug] [--features <list>] | check-elf <path>>".into()
}

fn project_root() -> PathBuf {
    // xtask/Cargo.toml lives one directory below the repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

fn cmd_bin(args: &[String]) -> Result<(), String> {
    let mut release = true;
    let mut features: Option<String> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--debug" => release = false,
            "--features" => {
                features = Some(iter.next().ok_or("--features needs a value")?.clone());
            }
            other => return Err(format!("unknown argument `{other}`\n{}", usage())),
        }
    }

    let root = project_root();
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .args(["build", "--target", TARGET, "--bin", BIN_NAME]);
    if release {
        cmd.arg("--release");
    }
    if let Some(f) = &features {
        cmd.args(["--features", f]);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if !status.success() {
        return Err("firmware build failed".into());
    }

    let profile = if release { "release" } else { "debug" };
    let elf_path = root
        .join("target")
        .join(TARGET)
        .join(profile)
        .join(BIN_NAME);
    let bin = check_elf(&elf_path)?;

    let out = root.join("target").join("firmware.bin");
    fs::write(&out, &bin).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    println!(
        "wrote {} ({} bytes, {:.1} KiB of {} KiB available)",
        out.display(),
        bin.len(),
        bin.len() as f64 / 1024.0,
        image::FIRMWARE_MAX_LEN / 1024
    );
    println!(
        "flash address: {:#010x} (see docs/en/flashing.md)",
        image::FIRMWARE_START
    );
    Ok(())
}

fn check_elf(path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let segments = elf::load_segments(&data)?;
    let bin = image::build(&segments)?;
    println!(
        "{}: {} loadable segment(s), bootloader range untouched",
        path.display(),
        segments.len()
    );
    Ok(bin)
}
