use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::{Builder, EntryType, Header};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=bun.lock");
    println!("cargo:rerun-if-env-changed=BUN");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH");
    let bun_path = resolve_bun_path();

    run_bun_install(&manifest_dir, &bun_path);

    let archive_name = format!("freq-ai-agents-{target_os}-{target_arch}.tar.gz");
    let archive_path = out_dir.join(&archive_name);
    create_archive(&manifest_dir, &archive_path, &bun_path).expect("create agent runtime archive");
    let archive_sha256 = sha256_file(&archive_path).expect("hash agent runtime archive");

    write_generated(
        &out_dir.join("agent_runtime_generated.rs"),
        &archive_name,
        &archive_path,
        &archive_sha256,
        &target_os,
        &target_arch,
    )
    .expect("write generated agent runtime metadata");
}

fn run_bun_install(manifest_dir: &Path, bun_path: &Path) {
    let mut cmd = Command::new(bun_path);
    cmd.arg("install");
    if manifest_dir.join("bun.lock").exists() {
        cmd.arg("--frozen-lockfile");
    }
    let status = cmd
        .current_dir(manifest_dir)
        .status()
        .unwrap_or_else(|err| panic!("failed to run `{}` install`: {err}", bun_path.display()));

    if !status.success() {
        panic!(
            "`{} install` failed with status {status}",
            bun_path.display()
        );
    }
}

fn create_archive(manifest_dir: &Path, archive_path: &Path, bun_path: &Path) -> io::Result<()> {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tar_gz = File::create(archive_path)?;
    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut archive = Builder::new(encoder);

    append_file(&mut archive, manifest_dir, "package.json")?;
    append_file(&mut archive, manifest_dir, "bun.lock")?;
    append_bun_runtime(&mut archive, bun_path)?;
    archive.append_dir_all("node_modules", manifest_dir.join("node_modules"))?;
    archive.finish()?;
    archive.into_inner()?.finish()?;
    Ok(())
}

fn append_file(
    archive: &mut Builder<GzEncoder<File>>,
    manifest_dir: &Path,
    relative: &str,
) -> io::Result<()> {
    archive.append_path_with_name(manifest_dir.join(relative), relative)
}

fn append_bun_runtime(archive: &mut Builder<GzEncoder<File>>, bun_path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        archive.append_path_with_name(bun_path, "bin/bun.exe")?;
        archive.append_path_with_name(bun_path, "bin/node.exe")?;
    }

    #[cfg(not(windows))]
    {
        archive.append_path_with_name(bun_path, "bin/bun")?;
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Symlink);
        header.set_mode(0o755);
        header.set_size(0);
        archive.append_link(&mut header, "bin/node", "bun")?;
    }

    Ok(())
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 1024 * 64];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn write_generated(
    path: &Path,
    archive_name: &str,
    archive_path: &Path,
    archive_sha256: &str,
    target_os: &str,
    target_arch: &str,
) -> io::Result<()> {
    let archive_path = archive_path.display().to_string().replace('\\', "\\\\");
    let short_hash = &archive_sha256[..12];
    let source = format!(
        r##"pub const TARGET_OS: &str = {target_os:?};
pub const TARGET_ARCH: &str = {target_arch:?};
pub const ARCHIVE_NAME: &str = {archive_name:?};
pub const ARCHIVE_SHA256: &str = {archive_sha256:?};
pub const ARCHIVE_SHORT_SHA256: &str = {short_hash:?};
pub const ARCHIVE_BYTES: &[u8] = include_bytes!(r#"{archive_path}"#);
"##
    );
    File::create(path)?.write_all(source.as_bytes())
}

fn resolve_bun_path() -> PathBuf {
    let bun = env::var_os("BUN").unwrap_or_else(|| OsString::from("bun"));
    let bun_path = PathBuf::from(&bun);

    if is_path_like(&bun_path) {
        return fs::canonicalize(&bun_path).unwrap_or_else(|err| {
            panic!("failed to resolve BUN path `{}`: {err}", bun_path.display())
        });
    }

    let Some(path) = env::var_os("PATH") else {
        panic!("PATH is not set, and BUN was not an explicit path");
    };

    for dir in env::split_paths(&path) {
        for candidate in executable_candidates(&bun) {
            let path = dir.join(candidate);
            if path.is_file() {
                return fs::canonicalize(&path)
                    .unwrap_or_else(|err| panic!("failed to resolve `{}`: {err}", path.display()));
            }
        }
    }

    panic!("failed to find Bun executable; install Bun or set BUN=/path/to/bun");
}

fn is_path_like(path: &Path) -> bool {
    path.is_absolute() || path.components().count() > 1
}

#[cfg(windows)]
fn executable_candidates(program: &OsString) -> Vec<OsString> {
    let mut candidates = vec![program.clone()];
    let program = program.to_string_lossy();
    if !program.to_ascii_lowercase().ends_with(".exe") {
        candidates.push(OsString::from(format!("{program}.exe")));
    }
    candidates
}

#[cfg(not(windows))]
fn executable_candidates(program: &OsString) -> Vec<OsString> {
    vec![program.clone()]
}
