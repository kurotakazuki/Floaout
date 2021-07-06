use crate::io::ReadExt;
use crate::Metadata;
use std::io::{Error, ErrorKind, Result};

pub struct WavMetadata {
    /// Number of sample frames
    pub frames: u32,
    // Format tag
    pub format_tag: FormatTag,
    /// Channels
    pub channels: u16,
    /// Samples per sec
    pub samples_per_sec: u32,
    /// Bits Per Sample
    pub bits_per_sample: u16,
}
impl Metadata for WavMetadata {
    fn read<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let check_fourcc = |reader: &mut R, val: &str| {
            let fourcc = reader.read_string_for::<4>()?;
            Self::return_invalid_data_if_not_equal(fourcc, val.to_string())
        };
        // Riff Chunk
        check_fourcc(reader, "RIFF")?;
        // File size - 8
        reader.read_bytes_for::<4>()?;
        check_fourcc(reader, "WAVE")?;

        // Other chunk
        let other_chunk = |reader: &mut R| -> Result<()> {
            let chunk_size: u32 = reader.read_le()?;
            let mut buf = vec![0; chunk_size as usize];
            reader.read_exact(&mut buf)?;

            Ok(())
        };

        // Fmt Chunk
        loop {
            let fourcc = reader.read_bytes_for::<4>()?;

            match fourcc {
                // fmt
                [0x66, 0x6D, 0x74, 0x20] => {
                    let fmt_size: u32 = reader.read_le()?;
                    Self::return_invalid_data_if_not_equal(fmt_size, 16)?;

                    let format_tag: FormatTag = reader.read_le::<u16>()?.into();
                    let channels: u16 = reader.read_le()?;
                    let samples_per_sec: u32 = reader.read_le()?;
                    let avg_bytes_per_sec: u32 = reader.read_le()?;
                    let block_align: u16 = reader.read_le()?;
                    let bits_per_sample: u16 = reader.read_le()?;

                    // Data Chunk
                    loop {
                        let fourcc = reader.read_bytes_for::<4>()?;
                        match fourcc {
                            [0x64, 0x61, 0x74, 0x61] => {
                                let data_size: u32 = reader.read_le()?;

                                let frames =
                                    Self::calculate_frames(data_size, channels, bits_per_sample);

                                let wav_metadata = Self {
                                    frames,
                                    format_tag,
                                    channels,
                                    samples_per_sec,
                                    bits_per_sample,
                                };

                                Self::return_invalid_data_if_not_equal(
                                    avg_bytes_per_sec,
                                    wav_metadata.avg_bytes_per_sec(),
                                )?;
                                Self::return_invalid_data_if_not_equal(
                                    block_align,
                                    wav_metadata.block_align(),
                                )?;

                                return Ok(wav_metadata);
                            }
                            _ => {
                                other_chunk(reader)?;
                            }
                        }
                    }
                }
                _ => {
                    other_chunk(reader)?;
                }
            }
        }
    }
    fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        todo!()
    }
}

impl WavMetadata {
    pub fn calculate_frames(data_size: u32, channels: u16, bits_per_sample: u16) -> u32 {
        data_size / (channels * bits_per_sample / 8) as u32
    }

    pub const fn frames(&self) -> u32 {
        self.frames
    }

    pub const fn format_tag(&self) -> FormatTag {
        self.format_tag
    }

    pub const fn channels(&self) -> u16 {
        self.channels
    }

    pub const fn samples_per_sec(&self) -> u32 {
        self.samples_per_sec
    }

    pub const fn bits_per_sample(&self) -> u16 {
        self.bits_per_sample
    }

    pub const fn bytes_per_sample(&self) -> u16 {
        self.bits_per_sample / 8
    }

    pub const fn block_align(&self) -> u16 {
        self.bytes_per_sample() * self.channels()
    }

    pub const fn avg_bytes_per_sec(&self) -> u32 {
        self.samples_per_sec() * self.block_align() as u32
    }

    fn return_invalid_data_if_not_equal<T: std::fmt::Display + Eq>(
        val: T,
        expect: T,
    ) -> Result<()> {
        if val != expect {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Expect {}", expect),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FormatTag {
    UncompressedPCM,
    IEEEFloatingPoint,
    // WaveFormatExtensible,
    Other(u16),
}

impl From<FormatTag> for u16 {
    fn from(format_tag: FormatTag) -> Self {
        match format_tag {
            FormatTag::UncompressedPCM => 1,
            FormatTag::IEEEFloatingPoint => 3,
            // FormatTag::WaveFormatExtensible => 65534,
            FormatTag::Other(n) => n,
        }
    }
}

impl From<u16> for FormatTag {
    fn from(n: u16) -> Self {
        match n {
            1 => FormatTag::UncompressedPCM,
            3 => FormatTag::IEEEFloatingPoint,
            // 65534 => FormatTag::WaveFormatExtensible,
            _ => FormatTag::Other(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata() {
        let format_tag = FormatTag::IEEEFloatingPoint;
        let frames = 0;
        let samples_per_sec = 44100;
        let bits_per_sample = 32;

        let metadata = WavMetadata {
            frames,
            format_tag,
            channels: 1,
            samples_per_sec,
            bits_per_sample,
        };
        assert_eq!(metadata.format_tag(), FormatTag::IEEEFloatingPoint);
        assert_eq!(metadata.channels(), 1);
        assert_eq!(metadata.samples_per_sec(), 44100);
        assert_eq!(metadata.avg_bytes_per_sec(), 176400);
        assert_eq!(metadata.block_align(), 4);
        assert_eq!(metadata.bits_per_sample(), 32);
        assert_eq!(metadata.bytes_per_sample(), 4);

        let metadata = WavMetadata {
            frames,
            format_tag,
            channels: 2,
            samples_per_sec,
            bits_per_sample,
        };
        assert_eq!(metadata.format_tag(), FormatTag::IEEEFloatingPoint);
        assert_eq!(metadata.channels(), 2);
        assert_eq!(metadata.samples_per_sec(), 44100);
        assert_eq!(metadata.avg_bytes_per_sec(), 352800);
        assert_eq!(metadata.block_align(), 8);
        assert_eq!(metadata.bits_per_sample(), 32);
        assert_eq!(metadata.bytes_per_sample(), 4);
    }
}
