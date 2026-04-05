use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

#[derive(Debug, Error)]
pub enum HFDiffParseError {
    #[error("diff line is empty")]
    EmptyLine,
    #[error("failed to parse file size from {value:?} in line {line:?}: {source}")]
    InvalidFileSize {
        value: String,
        line: String,
        source: std::num::ParseIntError,
    },
    #[error("incorrect diff line format: {line:?}")]
    InvalidFormat { line: String },
    #[error("I/O error while reading diff stream: {0}")]
    Io(#[from] std::io::Error),
}

/// The hash of Git's empty tree. Using it as the "old" tree in a diff returns
/// raw diff values for all files in the revision (file size, path, blob id,
/// binary flag).
pub const GIT_EMPTY_TREE_HASH: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HFFileDiff {
    pub old_blob_id: String,
    pub new_blob_id: String,
    pub status: GitStatus,
    pub file_path: String,
    pub new_file_path: Option<String>,
    pub is_binary: bool,
    pub new_file_size: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GitStatus {
    Addition,
    Copy,
    Deletion,
    Modification,
    FileTypeChange,
    Rename,
    Unknown,
    Unmerged,
}

impl From<char> for GitStatus {
    fn from(value: char) -> Self {
        match value {
            'A' => Self::Addition,
            'C' => Self::Copy,
            'D' => Self::Deletion,
            'M' => Self::Modification,
            'R' => Self::Rename,
            'T' => Self::FileTypeChange,
            'U' => Self::Unmerged,
            'X' => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

/// Parse a single line of HF raw diff output into an `HFFileDiff`.
///
/// Format reference: <https://git-scm.com/docs/diff-format#_raw_output_format>
pub fn parse_hf_diff_line(line: &str) -> Result<HFFileDiff, HFDiffParseError> {
    let original_line = line;
    let fmt_err = || HFDiffParseError::InvalidFormat {
        line: original_line.to_owned(),
    };
    let bin_or_text = line.chars().next().ok_or(HFDiffParseError::EmptyLine)?;
    let is_binary = bin_or_text != 'T';
    // skip {B|T} and space
    let line = line.get(2..).ok_or_else(&fmt_err)?;
    let mut i = 0;
    for char in line.chars() {
        if !char.is_ascii_digit() {
            break;
        }
        i += 1;
    }
    let size_str = &line[..i];
    let new_file_size = size_str.parse().map_err(|e| HFDiffParseError::InvalidFileSize {
        value: size_str.to_owned(),
        line: original_line.to_owned(),
        source: e,
    })?;
    // skip file size + \t
    let line = line.get(i + 1..).ok_or_else(&fmt_err)?;
    // skip :000000 000000 & space
    let line = line.get(15..).ok_or_else(&fmt_err)?;
    let old_blob_id = line.get(..40).ok_or_else(&fmt_err)?.to_owned();
    // skip sha1;0{40}, ... & space
    let line = line.get(44..).ok_or_else(&fmt_err)?;
    let new_blob_id = line.get(..40).ok_or_else(&fmt_err)?.to_owned();
    // skip sha1;0{40}, ... & space
    let line = line.get(44..).ok_or_else(&fmt_err)?;
    let status = line.chars().next().ok_or_else(&fmt_err)?.into();
    let line = line.get(1..).ok_or_else(&fmt_err)?;
    // skip optional score digits 1-3 chars & \t
    let mut i = 0;
    for char in line.chars() {
        if !char.is_ascii_digit() {
            break;
        }
        i += 1;
    }
    let line = line.get(i + 1..).ok_or_else(&fmt_err)?;
    let separator_is_tab = line.contains('\t');
    // read up to next space or newline
    let i = if matches!(status, GitStatus::Copy | GitStatus::Rename) {
        let mut i = 0;
        for char in line.chars() {
            match (separator_is_tab, char) {
                (true, '\t') => break,
                (false, ' ') => break,
                _ => (),
            }
            i += 1;
        }
        i
    } else {
        line.len()
    };
    let file_path = line[..i].to_owned();
    let line = &line[i..];
    let new_file_path = if !line.is_empty() {
        // skip separator
        let line = &line[1..];
        Some(line.to_owned())
    } else {
        None
    };

    Ok(HFFileDiff {
        old_blob_id,
        new_blob_id,
        status,
        file_path,
        new_file_path,
        is_binary,
        new_file_size,
    })
}

/// Parse a full raw HF diff string into a list of `HFFileDiff` entries.
///
/// Format reference: <https://git-scm.com/docs/diff-format#_raw_output_format>
pub fn parse_raw_diff(raw_diff: &str) -> Result<Vec<HFFileDiff>, HFDiffParseError> {
    raw_diff.lines().map(parse_hf_diff_line).collect()
}

/// Streaming version of [`parse_raw_diff`].
///
/// Takes a byte stream (e.g. from `HFRepository::get_raw_diff_stream`) and returns
/// a stream of `HFFileDiff` items, parsing each line as it arrives without buffering
/// the entire response.
pub fn stream_raw_diff<S, E>(byte_stream: S) -> impl Stream<Item = Result<HFFileDiff, HFDiffParseError>> + Unpin
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<std::io::Error>,
{
    let mapped_stream = Box::pin(byte_stream).map(|r| r.map_err(Into::into));
    let reader = StreamReader::new(mapped_stream);
    let buf_reader = BufReader::new(reader);
    let lines = buf_reader.lines();

    Box::pin(stream::unfold(lines, |mut lines| async move {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let result = parse_hf_diff_line(&line);
                if let Err(ref err) = result {
                    tracing::warn!(
                        line = %line,
                        error = %err,
                        "failed to parse diff line"
                    );
                }
                Some((result, lines))
            },
            Ok(None) => None,
            Err(e) => Some((Err(HFDiffParseError::Io(e)), lines)),
        }
    }))
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::{parse_hf_diff_line, parse_raw_diff, stream_raw_diff, GitStatus, HFFileDiff};

    #[test]
    fn modified_hf_diff() {
        let file_diffs = parse_raw_diff(
            r#"T 2305	:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... M	apps/scan_orchestrator/src/dispatcher.rs
T 4422	:100644 100644 c417bf5a3fbec60b22aeab13cfa4d9439155303b... 0000000000000000000000000000000000000000... M	apps/scan_orchestrator/src/git.rs
T 4591	:100644 100644 3435c69bc88a70105d6b864e5e462fac490ec2ff... 0000000000000000000000000000000000000000... M	apps/shared/src/gitaly/mod.rs
T 864	:100644 100644 cb1a9405e1ce054eca9aef81cdb782963a84a0b0... 0000000000000000000000000000000000000000... M	apps/shared/src/message_queue/rabbitmq.rs
T 452	:100644 100644 f7b95e09e0573a829c338fe46e451b5609424a70... 0000000000000000000000000000000000000000... M	apps/shared/src/scanner/file.rs"#,
        )
        .unwrap();
        assert_eq!(
            file_diffs,
            vec![
                HFFileDiff {
                    old_blob_id: "97e7432a448baa9e97ec5e4f03c57b09b8e116ed".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Modification,
                    file_path: "apps/scan_orchestrator/src/dispatcher.rs".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 2305,
                },
                HFFileDiff {
                    old_blob_id: "c417bf5a3fbec60b22aeab13cfa4d9439155303b".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Modification,
                    file_path: "apps/scan_orchestrator/src/git.rs".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 4422,
                },
                HFFileDiff {
                    old_blob_id: "3435c69bc88a70105d6b864e5e462fac490ec2ff".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Modification,
                    file_path: "apps/shared/src/gitaly/mod.rs".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 4591,
                },
                HFFileDiff {
                    old_blob_id: "cb1a9405e1ce054eca9aef81cdb782963a84a0b0".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Modification,
                    file_path: "apps/shared/src/message_queue/rabbitmq.rs".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 864,
                },
                HFFileDiff {
                    old_blob_id: "f7b95e09e0573a829c338fe46e451b5609424a70".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Modification,
                    file_path: "apps/shared/src/scanner/file.rs".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 452,
                }
            ]
        )
    }

    #[test]
    fn rename_and_copy_hf_diff() {
        let file_diffs = parse_raw_diff(
            r#"B 421211	:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... C68	apps/scan_orchestrator/src/dispatcher.rs apps/scan_orchestrator/src/blob
T 1679	:100644 100644 f7b95e09e0573a829c338fe46e451b5609424a70... 0000000000000000000000000000000000000000... R	apps/shared/src/scanner/file.rs apps/shared/src/scanner/file3.rs"#,
        )
        .unwrap();
        assert_eq!(
            file_diffs,
            vec![
                HFFileDiff {
                    old_blob_id: "97e7432a448baa9e97ec5e4f03c57b09b8e116ed".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Copy,
                    file_path: "apps/scan_orchestrator/src/dispatcher.rs".to_owned(),
                    new_file_path: Some("apps/scan_orchestrator/src/blob".to_owned()),
                    is_binary: true,
                    new_file_size: 421211,
                },
                HFFileDiff {
                    old_blob_id: "f7b95e09e0573a829c338fe46e451b5609424a70".to_owned(),
                    new_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    status: GitStatus::Rename,
                    file_path: "apps/shared/src/scanner/file.rs".to_owned(),
                    new_file_path: Some("apps/shared/src/scanner/file3.rs".to_owned()),
                    is_binary: false,
                    new_file_size: 1679,
                }
            ]
        )
    }

    #[test]
    fn special_chars() {
        let file_diffs = parse_raw_diff(
            r#"T 37861440	:000000 100644 0000000000000000000000000000000000000000... 30a03d21620ebc6167e350aef9e2ac2774cf372d... A	AI_popai/エイミ Eimi-ブルーアーカイブ Blue Archive (230270)/259889/eimi_(blue_archive).safetensors
T 228455604	:000000 100644 0000000000000000000000000000000000000000... 77367f06242f620081e0103c599818bfde8d4c75... D	Faeia/💀SDXL Antler Pagan💀 (236040)/266140/SDXLAntlerPagan.safetensors
T 228455084	:000000 100644 0000000000000000000000000000000000000000... c6c5a5a38c3c6eb2049e9727ad4ba8d1f252ef7c... D	Faeia/😡SDXL Rage Style😡 (234815)/264786/SDXLRageStyle.safetensors
T 228455220	:000000 100644 0000000000000000000000000000000000000000... fccf5af199707c0b50cd321fc17b1de38071290a... A	Faeia/🥩SDXL Elf Meat - A Reindeer Delicacy🥩 (231879)/261722/SDXLElfMeat.safetensors"#,
        )
        .unwrap();
        assert_eq!(
            file_diffs,
            vec![
                HFFileDiff {
                    old_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    new_blob_id: "30a03d21620ebc6167e350aef9e2ac2774cf372d".to_owned(),
                    status: GitStatus::Addition,
                    file_path: "AI_popai/エイミ Eimi-ブルーアーカイブ Blue Archive (230270)/259889/eimi_(blue_archive).safetensors".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 37861440,
                },
                HFFileDiff {
                    old_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    new_blob_id: "77367f06242f620081e0103c599818bfde8d4c75".to_owned(),
                    status: GitStatus::Deletion,
                    file_path: "Faeia/💀SDXL Antler Pagan💀 (236040)/266140/SDXLAntlerPagan.safetensors".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 228455604,
                },
                HFFileDiff {
                    old_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    new_blob_id: "c6c5a5a38c3c6eb2049e9727ad4ba8d1f252ef7c".to_owned(),
                    status: GitStatus::Deletion,
                    file_path: "Faeia/😡SDXL Rage Style😡 (234815)/264786/SDXLRageStyle.safetensors".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 228455084,
                },
                HFFileDiff {
                    old_blob_id: "0000000000000000000000000000000000000000".to_owned(),
                    new_blob_id: "fccf5af199707c0b50cd321fc17b1de38071290a".to_owned(),
                    status: GitStatus::Addition,
                    file_path: "Faeia/🥩SDXL Elf Meat - A Reindeer Delicacy🥩 (231879)/261722/SDXLElfMeat.safetensors".to_owned(),
                    new_file_path: None,
                    is_binary: false,
                    new_file_size: 228455220,
                }
            ]
        )
    }

    // A valid line used as a base for truncation tests.
    const VALID_LINE: &str = "T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... M\tapps/scan_orchestrator/src/dispatcher.rs";

    fn assert_invalid_format(input: &str) {
        match parse_hf_diff_line(input) {
            Err(super::HFDiffParseError::InvalidFormat { .. }) => {},
            other => panic!("expected InvalidFormat for {input:?}, got {other:?}"),
        }
    }

    #[test]
    fn empty_line_error() {
        assert!(matches!(parse_hf_diff_line(""), Err(super::HFDiffParseError::EmptyLine)));
    }

    #[test]
    fn single_char_truncated() {
        assert_invalid_format("T");
    }

    #[test]
    fn truncated_after_size() {
        assert_invalid_format("T 2305");
    }

    #[test]
    fn truncated_before_mode_pair() {
        assert_invalid_format("T 2305\t:100644");
    }

    #[test]
    fn truncated_before_old_blob_id() {
        assert_invalid_format("T 2305\t:100644 100644 abcdef");
    }

    #[test]
    fn truncated_before_new_blob_id() {
        assert_invalid_format("T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... ");
    }

    #[test]
    fn truncated_before_status() {
        assert_invalid_format(
            "T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000...",
        );
    }

    #[test]
    fn truncated_after_status() {
        assert_invalid_format(
            "T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... M",
        );
    }

    #[test]
    fn valid_line_parses_ok() {
        parse_hf_diff_line(VALID_LINE).unwrap();
    }

    #[tokio::test]
    async fn stream_modified_hf_diff() {
        let input = b"T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... M\tapps/scan_orchestrator/src/dispatcher.rs\nT 4422\t:100644 100644 c417bf5a3fbec60b22aeab13cfa4d9439155303b... 0000000000000000000000000000000000000000... M\tapps/scan_orchestrator/src/git.rs\n";

        let byte_stream = futures::stream::once(async { Ok::<_, std::io::Error>(bytes::Bytes::from(&input[..])) });

        let diffs: Vec<HFFileDiff> = stream_raw_diff(byte_stream).map(|r| r.unwrap()).collect().await;

        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].file_path, "apps/scan_orchestrator/src/dispatcher.rs");
        assert_eq!(diffs[0].status, GitStatus::Modification);
        assert_eq!(diffs[0].new_file_size, 2305);
        assert_eq!(diffs[1].file_path, "apps/scan_orchestrator/src/git.rs");
        assert_eq!(diffs[1].status, GitStatus::Modification);
        assert_eq!(diffs[1].new_file_size, 4422);
    }

    #[tokio::test]
    async fn stream_across_chunk_boundaries() {
        let chunk1 = b"T 2305\t:100644 100644 97e7432a448baa9e97ec5e4f03c57b09b8e116ed... 0000000000000000000000000000000000000000... M\tapps/scan_orchestrator/src/dispatcher.rs\nT 44";
        let chunk2 = b"22\t:100644 100644 c417bf5a3fbec60b22aeab13cfa4d9439155303b... 0000000000000000000000000000000000000000... M\tapps/scan_orchestrator/src/git.rs\n";

        let byte_stream = futures::stream::iter(vec![
            Ok::<_, std::io::Error>(bytes::Bytes::from(&chunk1[..])),
            Ok(bytes::Bytes::from(&chunk2[..])),
        ]);

        let diffs: Vec<HFFileDiff> = stream_raw_diff(byte_stream).map(|r| r.unwrap()).collect().await;

        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].file_path, "apps/scan_orchestrator/src/dispatcher.rs");
        assert_eq!(diffs[1].file_path, "apps/scan_orchestrator/src/git.rs");
    }
}
