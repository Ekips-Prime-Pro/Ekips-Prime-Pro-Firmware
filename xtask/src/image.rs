//! Builds the raw flash image and enforces the brick-protection rules.

use crate::elf::Segment;

/// First byte of the LEGO DFU bootloader.
pub const BOOTLOADER_START: u32 = 0x0800_0000;
/// First byte after the LEGO DFU bootloader = first byte of our firmware.
pub const FIRMWARE_START: u32 = 0x0800_8000;
/// First byte after the internal flash (1 MiB device).
pub const FLASH_END: u32 = 0x0810_0000;
/// Maximum firmware size in bytes.
pub const FIRMWARE_MAX_LEN: u32 = FLASH_END - FIRMWARE_START;

const RAM_START: u32 = 0x2000_0000;
const RAM_END: u32 = 0x2005_0000;

/// Concatenates the segments into one image that starts at
/// [`FIRMWARE_START`]. Gaps are filled with `0xFF` (erased flash).
///
/// Fails if any byte would land outside `FIRMWARE_START..FLASH_END`
/// (this includes the bootloader) or if the vector table looks wrong.
pub fn build(segments: &[Segment]) -> Result<Vec<u8>, String> {
    for s in segments {
        let start = s.paddr as u64;
        let end = start + s.data.len() as u64;
        if start < FIRMWARE_START as u64 && end > BOOTLOADER_START as u64 {
            return Err(format!(
                "segment at {start:#010x} overlaps the LEGO bootloader \
                 ({BOOTLOADER_START:#010x}..{FIRMWARE_START:#010x}); refusing to build an image"
            ));
        }
        if start < FIRMWARE_START as u64 || end > FLASH_END as u64 {
            return Err(format!(
                "segment {start:#010x}..{end:#010x} is outside the firmware flash range \
                 {FIRMWARE_START:#010x}..{FLASH_END:#010x}"
            ));
        }
    }

    let lowest = segments
        .iter()
        .map(|s| s.paddr)
        .min()
        .ok_or("no segments")?;
    if lowest != FIRMWARE_START {
        return Err(format!(
            "image starts at {lowest:#010x}, expected the vector table at {FIRMWARE_START:#010x}"
        ));
    }
    let highest = segments
        .iter()
        .map(|s| s.paddr + s.data.len() as u32)
        .max()
        .ok_or("no segments")?;

    let mut bin = vec![0xFFu8; (highest - FIRMWARE_START) as usize];
    for s in segments {
        let off = (s.paddr - FIRMWARE_START) as usize;
        bin[off..off + s.data.len()].copy_from_slice(&s.data);
    }

    check_vector_table(&bin)?;
    Ok(bin)
}

/// Sanity checks on the first two vector table entries.
fn check_vector_table(bin: &[u8]) -> Result<(), String> {
    if bin.len() < 8 {
        return Err("image too small to contain a vector table".into());
    }
    let sp = u32::from_le_bytes([bin[0], bin[1], bin[2], bin[3]]);
    let reset = u32::from_le_bytes([bin[4], bin[5], bin[6], bin[7]]);
    if !(RAM_START..=RAM_END).contains(&sp) {
        return Err(format!("initial stack pointer {sp:#010x} is not in RAM"));
    }
    if reset & 1 == 0 {
        return Err(format!("reset vector {reset:#010x} is not a Thumb address"));
    }
    let reset_addr = reset & !1;
    if !(FIRMWARE_START..FLASH_END).contains(&reset_addr) {
        return Err(format!(
            "reset vector {reset:#010x} does not point into the firmware"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_table() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0x2005_0000u32.to_le_bytes());
        v.extend_from_slice(&0x0800_8401u32.to_le_bytes());
        v
    }

    fn seg(paddr: u32, data: Vec<u8>) -> Segment {
        Segment { paddr, data }
    }

    #[test]
    fn builds_contiguous_image_with_gap_fill() {
        let bin = build(&[
            seg(FIRMWARE_START, vector_table()),
            seg(FIRMWARE_START + 12, vec![0xAA]),
        ])
        .unwrap();
        assert_eq!(bin.len(), 13);
        assert_eq!(&bin[8..12], &[0xFF; 4]);
        assert_eq!(bin[12], 0xAA);
    }

    #[test]
    fn rejects_bootloader_overlap() {
        let err = build(&[seg(0x0800_7FFC, vec![0; 8])]).unwrap_err();
        assert!(err.contains("bootloader"));
        let err = build(&[seg(BOOTLOADER_START, vector_table())]).unwrap_err();
        assert!(err.contains("bootloader"));
    }

    #[test]
    fn rejects_out_of_flash() {
        assert!(
            build(&[
                seg(FIRMWARE_START, vector_table()),
                seg(FLASH_END - 1, vec![0; 2])
            ])
            .is_err()
        );
        assert!(build(&[seg(RAM_START, vec![0; 4])]).is_err());
    }

    #[test]
    fn rejects_bad_vector_table() {
        let mut v = vector_table();
        v[4] = 0x00; // even reset vector
        assert!(build(&[seg(FIRMWARE_START, v)]).is_err());
    }
}
