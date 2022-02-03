use std::ffi::OsStr;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    build_dir("proto/domain")?;
    build_dir("proto")?;

    Ok(())
}

fn build_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let proto_ext: Option<&OsStr> = Some(OsStr::new("proto"));
    let proto_files = path
        .as_ref()
        .read_dir()?
        .filter_map(|result| result.ok())
        .filter(|a| match a.file_type() {
            Ok(ft) => ft.is_file() && a.path().extension() == proto_ext,
            Err(_) => false,
        })
        .map(|a| a.path())
        .collect::<Vec<_>>();

    Ok(tonic_build::configure()
        // .server_mod_attribute("attrs", "#[cfg(feature = \"server\")]")
        // .client_mod_attribute("attrs", "#[cfg(feature = \"client\")]")
        .compile(&proto_files, &[path])?)
}
