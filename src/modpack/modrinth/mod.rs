pub mod structs;

use async_zip::{
    base::write::EntryStreamWriter, error::Result, tokio::write::ZipFileWriter, Compression,
    ZipEntryBuilder,
};
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{canonicalize, read, DirEntry, File},
    io::{AsyncRead, AsyncSeek},
};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

use super::read_from_zip;

/// Read the `input`'s metadata file to a string
pub async fn read_metadata_file(
    input: impl AsyncRead + AsyncSeek + Unpin,
) -> Result<Option<String>> {
    read_from_zip(input, "modrinth.index.json").await
}

/// Create a Modrinth modpack at `output` using the provided `metadata` and optional `overrides`
pub async fn create(
    output: &Path,
    metadata: &str,
    overrides: Option<&Path>,
    additional_mods: Option<&Path>,
) -> Result<File> {
    let compression = Compression::Deflate;
    let mut writer = ZipFileWriter::new(File::create(output).await?.compat());
    writer
        .write_entry_whole(
            ZipEntryBuilder::new("modrinth.index.json".into(), compression),
            metadata.as_bytes(),
        )
        .await?;
    if let Some(overrides) = overrides {
        super::compress_dir(
            &mut writer,
            overrides.parent().unwrap(),
            &PathBuf::from("overrides"),
            compression,
        )
        .await?;
    }

    if let Some(path) = additional_mods {
        for entry in read_dir(path)?
            .flatten()
            .filter(|entry| entry.file_type().map(|e| e.is_file()).unwrap_or(false))
        {
            let entry = canonicalize(entry.path()).await?;
            writer
                .write_entry_whole(
                    ZipEntryBuilder::new(
                        PathBuf::from("overrides")
                            .join("mods")
                            .with_file_name(entry.file_name().unwrap())
                            .to_string_lossy()
                            .as_ref()
                            .into(),
                        compression,
                    ),
                    &read(entry).await?,
                )
                .await?;
        }
    }
    writer.close().await.map(Compat::into_inner)
}
