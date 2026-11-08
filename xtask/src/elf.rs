//! Minimal ELF32 little-endian program header reader.
//!
//! Only what is needed to turn the firmware ELF into a raw binary: the
//! `PT_LOAD` segments with their physical (load) address and file bytes.

/// One loadable segment as it will be placed in flash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Physical (load) address, i.e. where the bytes live in flash.
    pub paddr: u32,
    /// The bytes stored in the file for this segment.
    pub data: Vec<u8>,
}

const PT_LOAD: u32 = 1;
const EM_ARM: u16 = 40;

fn u16_at(d: &[u8], off: usize) -> Result<u16, String> {
    d.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| "truncated ELF file".to_string())
}

fn u32_at(d: &[u8], off: usize) -> Result<u32, String> {
    d.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "truncated ELF file".to_string())
}

/// Returns all `PT_LOAD` segments that carry file data (`p_filesz > 0`).
pub fn load_segments(d: &[u8]) -> Result<Vec<Segment>, String> {
    if d.len() < 52 || &d[0..4] != b"\x7fELF" {
        return Err("not an ELF file".into());
    }
    if d[4] != 1 || d[5] != 1 {
        return Err("expected a 32-bit little-endian ELF file".into());
    }
    if u16_at(d, 18)? != EM_ARM {
        return Err("expected an ARM ELF file (e_machine = EM_ARM)".into());
    }
    let phoff = u32_at(d, 28)? as usize;
    let phentsize = u16_at(d, 42)? as usize;
    let phnum = u16_at(d, 44)? as usize;
    if phentsize < 32 {
        return Err("invalid program header size".into());
    }

    let mut segments = Vec::new();
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        let p_type = u32_at(d, ph)?;
        let p_offset = u32_at(d, ph + 4)? as usize;
        let p_paddr = u32_at(d, ph + 12)?;
        let p_filesz = u32_at(d, ph + 16)? as usize;
        if p_type != PT_LOAD || p_filesz == 0 {
            continue;
        }
        let data = d
            .get(p_offset..p_offset + p_filesz)
            .ok_or("segment data outside of file")?
            .to_vec();
        segments.push(Segment {
            paddr: p_paddr,
            data,
        });
    }
    if segments.is_empty() {
        return Err("ELF file has no loadable segments".into());
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal ELF with the given (paddr, data) PT_LOAD segments.
    fn fake_elf(segs: &[(u32, &[u8])]) -> Vec<u8> {
        let mut d = vec![0u8; 52];
        d[0..4].copy_from_slice(b"\x7fELF");
        d[4] = 1;
        d[5] = 1;
        d[18..20].copy_from_slice(&EM_ARM.to_le_bytes());
        d[28..32].copy_from_slice(&52u32.to_le_bytes());
        d[42..44].copy_from_slice(&32u16.to_le_bytes());
        d[44..46].copy_from_slice(&(segs.len() as u16).to_le_bytes());
        let data_start = 52 + 32 * segs.len();
        let mut payload = Vec::new();
        for (paddr, data) in segs {
            let mut ph = [0u8; 32];
            ph[0..4].copy_from_slice(&PT_LOAD.to_le_bytes());
            let off = (data_start + payload.len()) as u32;
            ph[4..8].copy_from_slice(&off.to_le_bytes());
            ph[12..16].copy_from_slice(&paddr.to_le_bytes());
            ph[16..20].copy_from_slice(&(data.len() as u32).to_le_bytes());
            d.extend_from_slice(&ph);
            payload.extend_from_slice(data);
        }
        d.extend_from_slice(&payload);
        d
    }

    #[test]
    fn reads_segments() {
        let elf = fake_elf(&[(0x0800_8000, &[1, 2, 3, 4]), (0x0800_9000, &[5])]);
        let segs = load_segments(&elf).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].paddr, 0x0800_8000);
        assert_eq!(segs[1].data, vec![5]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(load_segments(b"hello").is_err());
    }
}
