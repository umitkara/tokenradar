//! Streams complete lines out of a byte range of a growing file.
//!
//! - Reads in [`CHUNK`]-sized blocks and finds newlines with `memchr`. Lines inside a block
//!   are handed out without copying; only lines that straddle two blocks are stitched together.
//! - A trailing line without `\n` is not handed out: Claude Code may still be writing it. The
//!   returned offset stops at the last newline, so the next pass starts at that line.
//! - Lines longer than [`MAX_LINE`] are skipped without being buffered. They are API request
//!   dumps of up to tens of MiB; buffering them took four workers to ~165 MiB (Step 02).

use std::io::{self, Read};
use std::ops::ControlFlow;

/// Read block size.
pub const CHUNK: usize = 1 << 20;

/// Longest line that is parsed. Assistant lines peak around 350 KiB (Step 02 profile).
pub const MAX_LINE: usize = 4 << 20;

/// Capacity the stitch buffer is shrunk back to after a long line.
const CARRY_KEEP: usize = 256 << 10;

/// Records that start like this (`api-request-blob`, …) carry no usage. An oversized line that
/// does not start like this might have been an assistant record and is counted as suspect.
const TYPED_HEAD: &[u8] = br#"{"type":""#;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReadStats {
    /// Bytes consumed up to the last complete line.
    pub bytes: u64,
    /// Complete lines handed out.
    pub lines: u64,
    /// Lines skipped for exceeding [`MAX_LINE`].
    pub oversized: u64,
    /// Oversized lines that did not start with a `type` field (see [`TYPED_HEAD`]).
    pub oversized_suspect: u64,
}

/// Reusable buffers; one per worker.
pub struct LineReader {
    block: Vec<u8>,
    carry: Vec<u8>,
}

impl Default for LineReader {
    fn default() -> Self {
        Self {
            block: vec![0; CHUNK],
            carry: Vec::new(),
        }
    }
}

impl LineReader {
    /// Reads `src`, positioned at absolute offset `start`, up to offset `end`.
    ///
    /// `on_line(line, next)` receives each complete line without its `\n` and the absolute
    /// offset just past it; returning `Break` stops early. Returns the absolute offset after
    /// the last complete line (or after the line that `Break` was returned for).
    pub fn read_range(
        &mut self,
        src: &mut impl Read,
        start: u64,
        end: u64,
        mut on_line: impl FnMut(&[u8], u64) -> ControlFlow<()>,
    ) -> io::Result<(u64, ReadStats)> {
        let Self { block, carry } = self;
        carry.clear();
        let mut stats = ReadStats::default();
        let mut pos = start;
        let mut complete = start;
        // Inside an oversized line: discard until its newline.
        let mut skipping = false;
        let mut skipped_head_typed = false;

        'read: while pos < end {
            let want = usize::try_from(end - pos).map_or(CHUNK, |r| r.min(CHUNK));
            let n = src.read(&mut block[..want])?;
            if n == 0 {
                break; // the file shrank after it was measured; the caller re-checks next pass
            }
            let data = &block[..n];
            let base = pos;
            pos += n as u64;

            let mut i = 0;
            while i < n {
                let Some(j) = memchr::memchr(b'\n', &data[i..]) else {
                    let rest = &data[i..];
                    if !skipping && carry.len() + rest.len() > MAX_LINE {
                        skipping = true;
                        skipped_head_typed = if carry.is_empty() {
                            rest.starts_with(TYPED_HEAD)
                        } else {
                            carry.starts_with(TYPED_HEAD)
                        };
                        carry.clear();
                    } else if !skipping {
                        carry.extend_from_slice(rest);
                    }
                    break;
                };
                let nl = i + j;
                let next = base + nl as u64 + 1;
                let flow = if skipping {
                    skipping = false;
                    stats.oversized += 1;
                    stats.oversized_suspect += u64::from(!skipped_head_typed);
                    ControlFlow::Continue(())
                } else if carry.is_empty() {
                    stats.lines += 1;
                    on_line(&data[i..nl], next)
                } else if carry.len() + j > MAX_LINE {
                    // The newline arrived, but the stitched line would still be too long.
                    stats.oversized += 1;
                    stats.oversized_suspect += u64::from(!carry.starts_with(TYPED_HEAD));
                    carry.clear();
                    ControlFlow::Continue(())
                } else {
                    carry.extend_from_slice(&data[i..nl]);
                    stats.lines += 1;
                    let flow = on_line(carry, next);
                    carry.clear();
                    flow
                };
                complete = next;
                i = nl + 1;
                if flow.is_break() {
                    break 'read;
                }
            }
        }
        carry.clear();
        if carry.capacity() > CARRY_KEEP {
            carry.shrink_to(CARRY_KEEP);
        }
        stats.bytes = complete - start;
        Ok((complete, stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(data: &[u8], start: u64, end: u64) -> (Vec<(Vec<u8>, u64)>, u64, ReadStats) {
        let mut out = Vec::new();
        let mut reader = LineReader::default();
        let mut src = &data[usize::try_from(start).expect("small")..];
        let (offset, stats) = reader
            .read_range(&mut src, start, end, |l, next| {
                out.push((l.to_vec(), next));
                ControlFlow::Continue(())
            })
            .expect("read");
        (out, offset, stats)
    }

    #[test]
    fn half_written_last_line_is_left_for_the_next_pass() {
        let data = b"one\ntwo\nthr";
        let (lines, offset, stats) = lines_of(data, 0, data.len() as u64);
        assert_eq!(lines, vec![(b"one".to_vec(), 4), (b"two".to_vec(), 8)]);
        assert_eq!(offset, 8);
        assert_eq!(stats.bytes, 8);

        // The writer finishes the line; the next pass starts at the saved offset.
        let data = b"one\ntwo\nthree\n";
        let (lines, offset, _) = lines_of(data, 8, data.len() as u64);
        assert_eq!(lines, vec![(b"three".to_vec(), 14)]);
        assert_eq!(offset, 14);
    }

    #[test]
    fn lines_across_block_boundaries_are_stitched() {
        let long = vec![b'x'; CHUNK + 10];
        let mut data = b"a\n".to_vec();
        data.extend_from_slice(&long);
        data.extend_from_slice(b"\nb\n");
        let (lines, offset, stats) = lines_of(&data, 0, data.len() as u64);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].0, long);
        assert_eq!(offset, data.len() as u64);
        assert_eq!(stats.oversized, 0);
    }

    #[test]
    fn oversized_lines_are_skipped_and_classified() {
        let mut data = Vec::new();
        data.extend_from_slice(br#"{"type":"api-request-blob","x":""#);
        data.resize(data.len() + MAX_LINE + 5, b'z');
        data.extend_from_slice(b"\"}\nok\n");
        let mut suspect = br#"{"parentUuid":"p","message":""#.to_vec();
        suspect.resize(suspect.len() + MAX_LINE + 5, b'z');
        data.extend_from_slice(&suspect);
        data.extend_from_slice(b"\"}\nlast\n");

        let (lines, offset, stats) = lines_of(&data, 0, data.len() as u64);
        assert_eq!(
            lines.iter().map(|l| l.0.as_slice()).collect::<Vec<_>>(),
            vec![b"ok".as_slice(), b"last".as_slice()]
        );
        assert_eq!(offset, data.len() as u64);
        assert_eq!((stats.oversized, stats.oversized_suspect), (2, 1));
    }

    #[test]
    fn end_limits_the_range_and_break_stops_early() {
        let data = b"a\nb\nc\n";
        let (lines, offset, _) = lines_of(data, 0, 4);
        assert_eq!(lines.len(), 2);
        assert_eq!(offset, 4);

        let mut reader = LineReader::default();
        let (offset, stats) = reader
            .read_range(&mut &data[..], 0, 6, |_, _| ControlFlow::Break(()))
            .expect("read");
        assert_eq!((offset, stats.lines), (2, 1));
    }
}
