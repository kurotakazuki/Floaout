use crate::bub::{
    functions::{parse, BubFns, BubFnsAST, BubFnsVariable},
    BubID,
};
use crate::io::{ReadExt, WriteExt};
use crate::{LpcmKind, Metadata, CRC_32K_4_2};
use mycrc::CRC;
use std::io::{ErrorKind, Read, Result, Write};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BubState {
    Head,
    Body,
    Stopped,
    Ended,
}

impl BubState {
    pub fn is_head(&self) -> bool {
        matches!(self, Self::Head)
    }

    pub fn is_body(&self) -> bool {
        matches!(self, Self::Body)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped)
    }

    pub fn is_ended(&self) -> bool {
        matches!(self, Self::Ended)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BubSampleKind {
    Lpcm,
    Expr(BubFnsAST),
}

impl From<BubFnsAST> for BubSampleKind {
    fn from(ast: BubFnsAST) -> Self {
        Self::Expr(ast)
    }
}

impl BubSampleKind {
    pub fn default_expr() -> Self {
        parse("0".as_bytes(), &BubFnsVariable::Sum).unwrap().into()
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let value: u8 = reader.read_le()?;
        Ok(match value {
            0 => Self::Lpcm,
            1 => Self::default_expr(),
            _ => return Err(ErrorKind::InvalidData.into()),
        })
    }
    pub fn read_and_calc_bytes<R: Read>(reader: &mut R, crc: &mut CRC<u32>) -> Result<Self> {
        let value: u8 = reader.read_le_and_calc_bytes(crc)?;
        Ok(match value {
            0 => Self::Lpcm,
            1 => Self::default_expr(),
            _ => return Err(ErrorKind::InvalidData.into()),
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_le(self.to_u8())
    }
    pub fn write_and_calc_bytes<W: Write>(&self, writer: &mut W, crc: &mut CRC<u32>) -> Result<()> {
        writer.write_le_and_calc_bytes(self.to_u8(), crc)
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Lpcm,
            1 => Self::default_expr(),
            _ => unimplemented!(),
        }
    }

    pub const fn to_u8(&self) -> u8 {
        match self {
            Self::Lpcm => 0,
            Self::Expr(_) => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BubbleMetadata {
    // In File Header
    /// Version of Bubble File Format Specification.
    pub spec_version: u8,
    /// Bubble ID
    pub bub_id: BubID,
    /// Version of Bubble
    pub bub_version: u16,
    /// Frames
    pub frames: u64,
    /// Samples Per Sec
    pub samples_per_sec: f64,
    /// Bits Per Sample
    pub lpcm_kind: LpcmKind,
    /// Bubble Sample Kind
    pub bub_sample_kind: BubSampleKind,
    /// Name of Bubble
    pub name: String,

    /// Bubble State
    pub bub_state: BubState,
    /// Head Absolute Frame
    pub head_absolute_frame: u64,

    /// Bubble Functions
    pub bub_functions: BubFns,
    /// Foot Absolute Frame Plus One
    pub foot_absolute_frame_plus_one: u64,
    /// Next Head Absolute Frame
    /// `None` if 0.
    pub next_head_absolute_frame: Option<u64>,

    /// CRC
    pub crc: CRC<u32>,
}

impl BubbleMetadata {
    pub const fn frames(&self) -> u64 {
        self.frames
    }

    pub const fn lpcm_kind(&self) -> LpcmKind {
        self.lpcm_kind
    }

    pub const fn samples_per_sec(&self) -> f64 {
        self.samples_per_sec
    }

    pub fn set_as_head(&mut self, pos: u64) {
        self.head_absolute_frame = pos;
        self.bub_state = BubState::Head;
    }

    pub fn set_as_body(&mut self) {
        self.bub_state = BubState::Body;
    }

    pub fn set_as_stopped(&mut self) {
        self.bub_state = BubState::Stopped;
    }

    pub fn set_as_ended(&mut self) {
        self.bub_state = BubState::Ended;
    }

    pub(crate) fn set_bub_state_from_connected_and_ended(&mut self, pos: u64) {
        // TODO: Create closures method
        match self.next_head_absolute_frame {
            Some(next_head_absolute_frame) => {
                if next_head_absolute_frame == pos {
                    self.set_as_head(pos)
                } else {
                    self.set_as_stopped()
                }
            }
            None => self.set_as_ended(),
        }
    }

    pub fn init_with_pos(&mut self, pos: u64) {
        match self.bub_state {
            BubState::Head => {
                if self.foot_absolute_frame_plus_one == pos {
                    self.set_bub_state_from_connected_and_ended(pos);
                } else {
                    self.set_as_body();
                }
            }
            BubState::Body => {
                if self.foot_absolute_frame_plus_one == pos {
                    self.set_bub_state_from_connected_and_ended(pos);
                }
            }
            BubState::Stopped => {
                if self.next_head_absolute_frame.expect("Some") == pos {
                    self.set_as_head(pos);
                }
            }
            BubState::Ended => (),
        }
    }

    pub(crate) fn set_next_head_absolute_frame_from_relative(
        &mut self,
        next_head_relative_frame: u64,
        pos: u64,
    ) {
        self.next_head_absolute_frame = if next_head_relative_frame != 0 {
            Some(next_head_relative_frame + pos - 1)
        } else {
            None
        };
    }
    // IO
    pub(crate) fn read_next_head_absolute_frame_from_relative<R: std::io::Read>(
        &mut self,
        reader: &mut R,
        pos: u64,
    ) -> Result<()> {
        let next_head_relative_frame: u64 = reader.read_le_and_calc_bytes(&mut self.crc)?;
        self.set_next_head_absolute_frame_from_relative(next_head_relative_frame, pos);

        Ok(())
    }

    pub(crate) fn read_crc<R: std::io::Read>(&mut self, reader: &mut R) -> Result<()> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        self.crc.calc_bytes(&buf);
        // TODO: Return Error
        assert!(self.crc.is_error_free());

        self.crc.initialize().calc_bytes(&buf);

        Ok(())
    }
    pub(crate) fn write_crc<W: std::io::Write>(&mut self, writer: &mut W) -> Result<()> {
        let checksum_bytes = self.crc.finalize_to_endian_bytes();
        writer.write_all(&checksum_bytes)?;

        self.crc.initialize().calc_bytes(&checksum_bytes);

        Ok(())
    }

    pub fn read<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let mut metadata = Self {
            spec_version: Default::default(),
            bub_id: Default::default(),
            bub_version: Default::default(),
            frames: Default::default(),
            samples_per_sec: Default::default(),
            lpcm_kind: LpcmKind::F64LE,
            bub_sample_kind: BubSampleKind::Lpcm,
            name: Default::default(),

            bub_state: BubState::Stopped,
            head_absolute_frame: 0,

            bub_functions: BubFns::new(),
            foot_absolute_frame_plus_one: 0,
            next_head_absolute_frame: None,

            crc: CRC_32K_4_2,
        };

        metadata.spec_version = reader.read_le_and_calc_bytes(&mut metadata.crc)?;
        metadata.bub_id = BubID::read_and_calc_bytes(reader, &mut metadata.crc)?;
        metadata.bub_version = reader.read_le_and_calc_bytes(&mut metadata.crc)?;

        metadata.frames = reader.read_le_and_calc_bytes(&mut metadata.crc)?;
        metadata.read_next_head_absolute_frame_from_relative(reader, 1)?;
        metadata.samples_per_sec = reader.read_le_and_calc_bytes(&mut metadata.crc)?;
        metadata.lpcm_kind = LpcmKind::read_and_calc_bytes(reader, &mut metadata.crc)?;
        metadata.bub_sample_kind = BubSampleKind::read_and_calc_bytes(reader, &mut metadata.crc)?;

        let name_size: u8 = reader.read_le_and_calc_bytes(&mut metadata.crc)?;
        metadata.name =
            reader.read_string_for_and_calc_bytes(name_size as usize, &mut metadata.crc)?;

        // CRC
        metadata.read_crc(reader)?;

        Ok(metadata)
    }

    fn next_head_absolute_frame_into_relative(&self, pos: u64) -> u64 {
        match self.next_head_absolute_frame {
            Some(n) => 1 + n - pos,
            None => 0,
        }
    }

    pub fn write<W: std::io::Write>(&mut self, writer: &mut W) -> Result<()> {
        writer.write_le_and_calc_bytes(self.spec_version, &mut self.crc)?;
        self.bub_id.write_and_calc_bytes(writer, &mut self.crc)?;
        writer.write_le_and_calc_bytes(self.bub_version, &mut self.crc)?;

        writer.write_le_and_calc_bytes(self.frames, &mut self.crc)?;
        writer.write_le_and_calc_bytes(
            self.next_head_absolute_frame_into_relative(1),
            &mut self.crc,
        )?;
        writer.write_le_and_calc_bytes(self.samples_per_sec, &mut self.crc)?;
        self.lpcm_kind.write_and_calc_bytes(writer, &mut self.crc)?;
        self.bub_sample_kind
            .write_and_calc_bytes(writer, &mut self.crc)?;
        writer.write_le_and_calc_bytes(self.name.len() as u8, &mut self.crc)?;
        writer.write_str_and_calc_bytes(&self.name, &mut self.crc)?;

        // CRC
        self.write_crc(writer)
    }
}

impl Metadata for BubbleMetadata {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read() -> Result<()> {
        let mut bub_metadata = BubbleMetadata {
            spec_version: 0,
            bub_id: BubID::new(0),
            bub_version: 0,
            frames: 96000,
            samples_per_sec: 96000.0,
            lpcm_kind: LpcmKind::F32LE,
            bub_sample_kind: BubSampleKind::Lpcm,
            name: String::from("Vocal"),

            bub_state: BubState::Stopped,
            head_absolute_frame: 0,

            bub_functions: BubFns::new(),
            foot_absolute_frame_plus_one: 0,
            next_head_absolute_frame: Some(1),

            crc: CRC_32K_4_2,
        };
        let expected = bub_metadata.clone();

        let mut v: Vec<u8> = Vec::new();

        bub_metadata.write(&mut v)?;

        let mut val = BubbleMetadata::read(&mut &v[..])?;
        val.crc = CRC_32K_4_2;

        assert_eq!(val, expected);

        Ok(())
    }
}
