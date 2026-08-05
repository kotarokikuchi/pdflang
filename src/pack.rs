//! Comandos `pdfl pack` e `pdfl add`.
//! Pacote .pdflpkg = tar.gz determinístico com manifest.json (nome, versão
//! e SHA-256 de cada arquivo). `add` instala de um arquivo local, conferindo
//! os hashes. Fora desta fatia: repositório remoto e assinatura digital.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Extensões empacotadas: scripts e datasets.
const EXTENSIONS: &[&str] = &["pdfl", "csv", "txt", "json", "xlsx"];

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
    collect(folder, folder, &mut paths)?;
    paths.sort(); // ordem determinística
    if paths.is_empty() {
        bail!("no packable file in {} (extensions: {})", folder.display(), EXTENSIONS.join(", "));
    }

    let mut files = Vec::new();
    for rel in &paths {
        let bytes = std::fs::read(folder.join(rel))?;
        files.push(ManifestFile {
            path: rel.to_string_lossy().into_owned(),
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
        add_entry(&rel.to_string_lossy(), &bytes)?;
    }
    tar.into_inner()?.finish()?;
    Ok(manifest)
}

/// Instala um pacote local em `dir/<nome>@<versão>/`, conferindo os hashes.
pub fn add(package: &Path, dir: &Path) -> Result<(Manifest, PathBuf)> {
    let file = std::fs::File::open(package)
        .with_context(|| format!("could not open {}", package.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));

    // Lê tudo em memória (pacotes são pequenos: scripts + tabelas)
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

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e))
        {
            out.push(path.strip_prefix(root)?.to_path_buf());
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
    fn roundtrip_com_verificacao() {
        let src = tempdir("src");
        std::fs::write(src.join("perfil.pdfl"), "check \"a\" { require doc.page_count > 0 }").unwrap();
        std::fs::create_dir_all(src.join("dados")).unwrap();
        std::fs::write(src.join("dados/lotes.csv"), "a,b\n1,2\n").unwrap();
        std::fs::write(src.join("ignorado.exe"), "binário").unwrap();

        let pkg = tempdir("pkg").join("perfil.pdflpkg");
        let manifest = pack(&src, &pkg, "perfil-teste", "1.2.0").unwrap();
        assert_eq!(manifest.files.len(), 2); // .exe fora

        // determinismo: empacotar de novo dá o mesmo byte a byte
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

    #[test]
    fn detecta_adulteracao() {
        let src = tempdir("tamper-src");
        std::fs::write(src.join("perfil.pdfl"), "check \"a\" { }").unwrap();
        let pkg = tempdir("tamper-pkg").join("p.pdflpkg");
        pack(&src, &pkg, "p", "1.0").unwrap();

        // reconstrói o pacote mantendo o manifest original, mas trocando o
        // conteúdo do arquivo — o hash deixa de conferir
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
