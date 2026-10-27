//! A minimal zip writer: stored (uncompressed) entries only. A diagnostics bundle is a few
//! dozen kilobytes of text, and a compressor would add size to the 3 MB CLI for nothing.

use std::io::{self, Write};

/// Writes `files` as a zip archive. `dos_time` is `(date, time)` in MS-DOS format.
pub fn write(
    mut out: impl Write,
    files: &[(String, Vec<u8>)],
    dos_time: (u16, u16),
) -> io::Result<()> {
    let (date, time) = dos_time;
    let mut central = Vec::new();
    let mut offset: u32 = 0;
    for (name, data) in files {
        let crc = crc32(data);
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;
        let mut local = Vec::with_capacity(30 + name.len());
        local.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        local.extend_from_slice(&20_u16.to_le_bytes()); // version needed: 2.0
        local.extend_from_slice(&0x0800_u16.to_le_bytes()); // flags: UTF-8 names
        local.extend_from_slice(&0_u16.to_le_bytes()); // method: stored
        local.extend_from_slice(&time.to_le_bytes());
        local.extend_from_slice(&date.to_le_bytes());
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&name_len.to_le_bytes());
        local.extend_from_slice(&0_u16.to_le_bytes()); // extra length
        local.extend_from_slice(name.as_bytes());
        out.write_all(&local)?;
        out.write_all(data)?;

        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20_u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0x0800_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&time.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&[0; 8]); // extra, comment, disk, internal attributes
        central.extend_from_slice(&0_u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());

        let entry = u32::try_from(local.len() + data.len()).map_err(|_| too_big())?;
        offset = offset.checked_add(entry).ok_or_else(too_big)?;
    }
    let count = u16::try_from(files.len()).map_err(|_| too_big())?;
    let central_len = u32::try_from(central.len()).map_err(|_| too_big())?;
    out.write_all(&central)?;
    let mut end = Vec::with_capacity(22);
    end.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    end.extend_from_slice(&[0; 4]); // disk numbers
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&central_len.to_le_bytes());
    end.extend_from_slice(&offset.to_le_bytes());
    end.extend_from_slice(&0_u16.to_le_bytes()); // comment length
    out.write_all(&end)
}

fn too_big() -> io::Error {
    io::Error::other("diagnostics bundle too large for a zip32 archive")
}

/// MS-DOS `(date, time)` for a UTC civil time; years before 1980 clamp to 1980.
pub fn dos_time(dt: jiff::civil::DateTime) -> (u16, u16) {
    let year = u16::try_from(dt.year().clamp(1980, 2107) - 1980).unwrap_or(0);
    let field = |v: i8| u16::try_from(v).unwrap_or(0);
    let date = (year << 9) | (field(dt.month()) << 5) | field(dt.day());
    let time = (field(dt.hour()) << 11) | (field(dt.minute()) << 5) | (field(dt.second()) / 2);
    (date, time)
}

/// CRC-32 (IEEE), as zip requires.
fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            #[allow(clippy::cast_possible_truncation)]
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 == 1 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    let mut crc = !0u32;
    for &b in data {
        crc = TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn archive_layout_is_well_formed() {
        let files = vec![
            ("a.txt".to_owned(), b"hello".to_vec()),
            ("dir/b.json".to_owned(), b"{}".to_vec()),
        ];
        let mut buf = Vec::new();
        write(&mut buf, &files, (0x5B5B, 0x5000)).expect("write");
        // Local header, data, local header, data, two central entries, end record.
        let local = (30 + 5 + 5) + (30 + 10 + 2);
        let central = (46 + 5) + (46 + 10);
        assert_eq!(buf.len(), local + central + 22);
        assert_eq!(&buf[..4], b"PK\x03\x04");
        assert_eq!(&buf[30..35], b"a.txt");
        assert_eq!(&buf[35..40], b"hello");
        let end = &buf[buf.len() - 22..];
        assert_eq!(&end[..4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([end[10], end[11]]), 2);
        let cd_offset = u32::from_le_bytes([end[16], end[17], end[18], end[19]]) as usize;
        assert_eq!(cd_offset, local);
        assert_eq!(&buf[cd_offset..cd_offset + 4], b"PK\x01\x02");
    }

    #[test]
    fn dos_time_packs_fields() {
        let dt = jiff::civil::date(2026, 10, 27).at(21, 33, 58, 0);
        assert_eq!(
            dos_time(dt),
            ((46 << 9) | (10 << 5) | 0x1B, (21 << 11) | (33 << 5) | 0x1D)
        );
    }
}
