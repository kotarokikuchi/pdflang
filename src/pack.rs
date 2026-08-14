//! The `pdfl pack` and `pdfl add` commands.
//! A .pdflpkg package is a deterministic tar.gz with a manifest.json (name,
//! version and SHA-256 of each file). `add` installs from a local file,
//! checking the hashes. Out of scope here: remote repository and signing.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Packaged extensions: scripts, and the datasets `data::` can actually open.
///
/// `xlsx` used to be here. Packaging a file no `data::` function can read ships
/// a package that installs cleanly and then fails at the first lookup, so it is
/// refused at pack time instead — where the person who can fix it is standing.
const EXTENSIONS: &[&str] = &["pdfl", "csv", "txt", "json"];

/// Extensions that look like datasets but cannot be read, so a folder holding
/// one is a mistake worth naming rather than a file worth skipping quietly.
const UNREADABLE: &[&str] = &["xlsx", "xls", "ods"];

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
}

pub fn pack(folder: &Path, output: &Path, name: &str, version: &str) -> Result<Manifest> {
    let mut paths = Vec::new();
    let mut unreadable = Vec::new();
    collect(folder, folder, &mut paths, &mut unreadable)?;
    paths.sort(); // deterministic order
    unreadable.sort();
    for path in &unreadable {
        crate::note(format!(
            "not packaged: {} — no data:: function reads that format, and a package \
             that installs and then fails is worse than one file short",
            path.display()
        ));
    }
    if paths.is_empty() {
        bail!("no packable file in {} (extensions: {})", folder.display(), EXTENSIONS.join(", "));
    }

    // A package is an interchange format: it is built on one machine and
    // installed on another. `Path` renders with the separator of whichever
    // machine built it, so on Windows both the manifest and the archive
    // recorded `data\batches.csv`, which no other platform — and not our own
    // verification — would find again. Tar and zip have used `/` all along.
    fn slash_path(rel: &Path) -> String {
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    }

    let mut files = Vec::new();
    for rel in &paths {
        let bytes = std::fs::read(folder.join(rel))?;
        files.push(ManifestFile {
            path: slash_path(rel),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        });
    }
    let manifest = Manifest { name: name.into(), version: version.into(), files };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;

    let file = std::fs::File::create(output)
        .with_context(|| format!("could not create {}", output.display()))?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);

    let mut add_entry = |path: &str, data: &[u8]| -> Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0); // determinismo
        header.set_cksum();
        tar.append_data(&mut header, path, data)?;
        Ok(())
    };
    add_entry("manifest.json", manifest_json.as_bytes())?;
    for rel in &paths {
        let bytes = std::fs::read(folder.join(rel))?;
        add_entry(&slash_path(rel), &bytes)?;
    }
    tar.into_inner()?.finish()?;
    Ok(manifest)
}

/// Installs a local package into `dir/<name>@<version>/`, checking the hashes.
pub fn add(package: &Path, dir: &Path) -> Result<(Manifest, PathBuf)> {
    let file = std::fs::File::open(package)
        .with_context(|| format!("could not open {}", package.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));

    // Reads everything into memory (packages are small: scripts + tables)
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        if path.contains("..") || path.starts_with('/') {
            bail!("suspicious path in package: {path}");
        }
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data)?;
        entries.push((path, data));
    }

    let manifest_bytes = entries
        .iter()
        .find(|(p, _)| p == "manifest.json")
        .map(|(_, d)| d.clone())
        .context("package without manifest.json")?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).context("invalid manifest.json")?;

    let target = dir.join(format!("{}@{}", manifest.name, manifest.version));
    std::fs::create_dir_all(&target)?;

    for mf in &manifest.files {
        let (_, data) = entries
            .iter()
            .find(|(p, _)| *p == mf.path)
            .with_context(|| format!("file {} listed in the manifest but missing from the package", mf.path))?;
        let hash = format!("{:x}", Sha256::digest(data));
        if hash != mf.sha256 {
            bail!("hash of {} does not match (corrupted or tampered package)", mf.path);
        }
        let dest = target.join(&mf.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, data)?;
    }
    Ok((manifest, target))
}

fn collect(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    unreadable: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, out, unreadable)?;
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if EXTENSIONS.contains(&ext.as_str()) {
            out.push(path.strip_prefix(root)?.to_path_buf());
        } else if UNREADABLE.contains(&ext.as_str()) {
            unreadable.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pdfl-pack-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_with_verification() {
        let src = tempdir("src");
        std::fs::write(src.join("perfil.pdfl"), "check \"a\" { require doc.page_count > 0 }").unwrap();
        std::fs::create_dir_all(src.join("dados")).unwrap();
        std::fs::write(src.join("dados/lotes.csv"), "a,b\n1,2\n").unwrap();
        std::fs::write(src.join("ignored.exe"), "binary").unwrap();

        let pkg = tempdir("pkg").join("perfil.pdflpkg");
        let manifest = pack(&src, &pkg, "perfil-teste", "1.2.0").unwrap();
        assert_eq!(manifest.files.len(), 2); // .exe fora

        // A nested entry is recorded with `/`, whatever the machine that built
        // it calls a separator. Windows recorded `dados\lotes.csv`, which the
        // verification below then could not find — the package installed and
        // was useless. This assertion is trivially true on Unix and is the
        // whole point of the test on Windows.
        let recorded: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert!(recorded.contains(&"dados/lotes.csv"), "recorded as: {recorded:?}");
        assert!(!recorded.iter().any(|p| p.contains('\\')), "recorded as: {recorded:?}");

        // determinism: packing again gives the same bytes
        let pkg2 = tempdir("pkg2").join("p2.pdflpkg");
        pack(&src, &pkg2, "perfil-teste", "1.2.0").unwrap();
        assert_eq!(std::fs::read(&pkg).unwrap(), std::fs::read(&pkg2).unwrap());

        let dest = tempdir("dest");
        let (m, target) = add(&pkg, &dest).unwrap();
        assert_eq!(m.name, "perfil-teste");
        assert!(target.ends_with("perfil-teste@1.2.0"));
        assert!(target.join("perfil.pdfl").exists());
        assert_eq!(std::fs::read_to_string(target.join("dados/lotes.csv")).unwrap(), "a,b\n1,2\n");
    }

    /// A spreadsheet next to the exported CSV is a normal thing to have, so it
    /// must not block the package — but it must not travel inside it either,
    /// because nothing can open it once installed.
    #[test]
    fn a_spreadsheet_stays_out_of_the_package() {
        let src = tempdir("xlsx-src");
        std::fs::write(src.join("perfil.pdfl"), "check \"a\" { }").unwrap();
        std::fs::write(src.join("lotes.csv"), "a,b\n").unwrap();
        std::fs::write(src.join("lotes.json"), "[[\"a\",\"b\"]]").unwrap();
        std::fs::write(src.join("lotes.xlsx"), "PK\x03\x04").unwrap();

        let pkg = tempdir("xlsx-pkg").join("p.pdflpkg");
        let manifest = pack(&src, &pkg, "p", "1.0").unwrap();
        let packed: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(packed, ["lotes.csv", "lotes.json", "perfil.pdfl"]);
    }

    #[test]
    fn detects_tampering() {
        let src = tempdir("tamper-src");
        std::fs::write(src.join("perfil.pdfl"), "check \"a\" { }").unwrap();
        let pkg = tempdir("tamper-pkg").join("p.pdflpkg");
        pack(&src, &pkg, "p", "1.0").unwrap();

        // rebuilds the package keeping the original manifest but swapping the
        // file's content — the hash stops matching
        let file = std::fs::File::open(&pkg).unwrap();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().into_owned();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut data).unwrap();
            entries.push((path, data));
        }
        let evil = tempdir("tamper-evil").join("evil.pdflpkg");
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&evil).unwrap(),
            flate2::Compression::default(),
        );
        let mut tar = tar::Builder::new(gz);
        for (path, data) in &entries {
            let payload: &[u8] =
                if path == "perfil.pdfl" { b"check \"ADULTERADO\" { }" } else { data };
            let mut header = tar::Header::new_gnu();
            header.set_size(payload.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_cksum();
            tar.append_data(&mut header, path.as_str(), payload).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();

        let dest = tempdir("tamper-dest");
        let err = add(&evil, &dest).unwrap_err();
        assert!(err.to_string().contains("does not match"), "{err}");
    }
}
