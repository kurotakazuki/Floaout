use crate::bub::{
    functions::{parse, BubFnsAST, BubFnsInterpreter, BubFnsVariable},
    BubMetadata, BubSampleKind, BubState,
};
use crate::io::ReadExt;
use crate::{Coord, Frame, FrameIOKind, FrameReader, Sample};
use std::io::{Read, Result};
use std::marker::PhantomData;

pub struct BubFrameReader<R: Read, S: Sample> {
    pub inner: R,
    pub pos: u64,
    _phantom_sample: PhantomData<S>,
    pub metadata: BubMetadata,
    /// Speakers absolute coordinates
    pub speakers_absolute_coord: Vec<Coord>,
}

impl<R: Read, S: Sample> FrameReader<R> for BubFrameReader<R, S> {
    fn get_ref(&self) -> &R {
        &self.inner
    }
    fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read, S: Sample> BubFrameReader<R, S> {
    pub fn new(inner: R, metadata: BubMetadata, speakers_absolute_coord: Vec<Coord>) -> Self {
        Self {
            inner,
            pos: 0,
            _phantom_sample: PhantomData,
            metadata,
            speakers_absolute_coord,
        }
    }

    fn read_head_metadata_and_calc_bytes(&mut self) -> Result<()> {
        let functions_size: u16 = self.inner.read_le_and_calc_bytes(&mut self.metadata.crc)?;

        let bub_functions_vec = self
            .inner
            .read_vec_for_and_calc_bytes(functions_size as usize, &mut self.metadata.crc)?;

        self.metadata.bub_functions = parse(&bub_functions_vec, &BubFnsVariable::BubFns)
            .unwrap()
            .into_original()
            .unwrap()
            .into_bub_functions()
            .unwrap();
        // Foot relative frame
        self.metadata.foot_absolute_frame_plus_one = self.pos
            + self
                .inner
                .read_le_and_calc_bytes::<u64>(&mut self.metadata.crc)?;
        // Next head relative frame
        self.metadata
            .read_next_head_absolute_frame_from_relative(&mut self.inner, self.pos)?;

        Ok(())
    }

    fn read_lpcm_and_crc(&mut self) -> Result<S> {
        let sample = S::read_and_calc_bytes(&mut self.inner, &mut self.metadata.crc)?;
        // Read CRC
        if self.metadata.foot_absolute_frame_plus_one - 1 == self.pos {
            self.metadata.read_crc(&mut self.inner)?;
        }

        Ok(sample)
    }

    fn read_expression_and_crc(&mut self) -> Result<Vec<u8>> {
        let expr_size: u16 = self.inner.read_le_and_calc_bytes(&mut self.metadata.crc)?;
        let expr = self
            .inner
            .read_vec_for_and_calc_bytes(expr_size as usize, &mut self.metadata.crc)?;
        // CRC
        self.metadata.read_crc(&mut self.inner)?;
        Ok(expr)
    }

    fn get_volume_and_interpreter(
        &self,
        speaker_absolute_coord: Coord,
    ) -> Option<Vec<(f64, BubFnsInterpreter)>> {
        self.metadata.bub_functions.to_volume(
            speaker_absolute_coord,
            self.pos as f64,
            (self.pos - self.metadata.head_absolute_frame + 1) as f64,
            self.metadata.frames as f64,
            self.metadata.samples_per_sec,
        )
    }

    fn read_lpcm_frame(&mut self, frame: &mut Frame<S>) -> Result<()> {
        let sample = self.read_lpcm_and_crc()?;

        if sample != S::default() {
            // TODO: Create method
            for (i, speaker_absolute_coord) in self.speakers_absolute_coord.iter().enumerate() {
                if let Some(volume_and_interpreter_vec) =
                    self.get_volume_and_interpreter(*speaker_absolute_coord)
                {
                    for (volume, _) in volume_and_interpreter_vec {
                        frame.0[i] += sample * S::from_f64(volume);
                    }
                }
            }
        }

        Ok(())
    }

    fn expr_frame(&self, expr: &BubFnsAST, frame: &mut Frame<S>) {
        for (i, speaker_absolute_coord) in self.speakers_absolute_coord.iter().enumerate() {
            if let Some(volume_and_interpreter_vec) =
                self.get_volume_and_interpreter(*speaker_absolute_coord)
            {
                for (volume, interpreter) in volume_and_interpreter_vec {
                    let sample = interpreter.eval_sum(expr).unwrap();
                    frame.0[i] += S::from_f64(sample * volume);
                }
            }
        }
    }
}

impl<R: Read, S: Sample> Iterator for BubFrameReader<R, S> {
    type Item = Result<Frame<S>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.metadata.frames() <= self.pos {
            return None;
        } else {
            self.pos += 1;
        }

        self.metadata.init_with_pos(self.pos);

        let channels = self.speakers_absolute_coord.len();

        let mut frame: Frame<S> = vec![S::default(); channels].into();

        match self.metadata.bub_state {
            BubState::Head => {
                if let Err(e) = self.read_head_metadata_and_calc_bytes() {
                    return Some(Err(e));
                }

                // Read Sample
                match self.metadata.bub_sample_kind {
                    BubSampleKind::Lpcm => {
                        if let Err(e) = self.read_lpcm_frame(&mut frame) {
                            return Some(Err(e));
                        }
                    }
                    BubSampleKind::Expr(_) => {
                        let expr = match self.read_expression_and_crc() {
                            Ok(v) => v,
                            Err(e) => return Some(Err(e)),
                        };
                        let expr = parse(&expr, &BubFnsVariable::Sum).unwrap();
                        self.expr_frame(&expr, &mut frame);
                        self.metadata.bub_sample_kind = expr.into();
                    }
                }
            }
            BubState::Body => {
                // Read Sample
                match &self.metadata.bub_sample_kind {
                    BubSampleKind::Lpcm => {
                        if let Err(e) = self.read_lpcm_frame(&mut frame) {
                            return Some(Err(e));
                        }
                    }
                    BubSampleKind::Expr(expr) => self.expr_frame(expr, &mut frame),
                }
            }
            BubState::Stopped => (),
            BubState::Ended => (),
        }

        Some(Ok(frame))
    }
}

pub type BubFrameReaderKind<R> = FrameIOKind<BubFrameReader<R, f32>, BubFrameReader<R, f64>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bub::{functions::BubFns, BubID, BubSampleKind, BubState, BubState::*};
    use crate::LpcmKind;

    #[test]
    fn read_lpcm_frames() {
        let mut metadata = BubMetadata {
            spec_version: 0,
            bub_id: BubID::new(0),
            bub_version: 0,
            frames: 8,
            samples_per_sec: 96000.0,
            lpcm_kind: LpcmKind::F32LE,
            bub_sample_kind: BubSampleKind::Lpcm,
            name: String::from("0.1*N"),

            bub_state: BubState::Stopped,
            head_absolute_frame: 0,

            bub_functions: BubFns::new(),
            foot_absolute_frame_plus_one: 0,
            next_head_absolute_frame: Some(1),

            crc: crate::crc::CRC_32K_4_2,
        };

        let speakers_absolute_coord = vec![(0.0, 0.0, 0.0).into(), (3.0, 0.0, 0.0).into()];

        // Write Metadata and get CRC
        let mut skip: Vec<u8> = Vec::new();
        metadata.write(&mut skip).unwrap();

        let data: &[u8] = &[
            // Frame 1
            &15u16.to_le_bytes()[..],
            b"1 2 3 X<3 0.1*N",
            &2u64.to_le_bytes(),
            &3u64.to_le_bytes(),
            &1.0f32.to_le_bytes(),
            // Frame 2
            &1.0f32.to_le_bytes(),
            &[253, 24, 123, 85], // crc
            // Frame 3
            &11u16.to_le_bytes()[..],
            b"1 2 3 X<3 1",
            &1u64.to_le_bytes(),
            &3u64.to_le_bytes(),
            &0.3f32.to_le_bytes(),
            &[250, 147, 10, 142], // crc
            // Frame 4

            // Frame 5
            &12u16.to_le_bytes()[..],
            b"0 0 0 0==0 1",
            &1u64.to_le_bytes(),
            &2u64.to_le_bytes(),
            &0.4f32.to_le_bytes(),
            &[84, 232, 255, 6], // crc
            // Frame 6
            &13u16.to_le_bytes()[..],
            b"0 0 n X>=3 -z",
            &1u64.to_le_bytes(),
            &0u64.to_le_bytes(),
            &1.0f32.to_le_bytes(),
            &[227, 183, 22, 42], // crc
                                 // Frame 7

                                 // Frame 8
        ]
        .concat();

        let mut bub_frame_reader: BubFrameReader<&[u8], f32> =
            BubFrameReader::new(data, metadata, speakers_absolute_coord);

        let expects = vec![
            (Head, [0.1, 0.0]),
            (Body, [0.2, 0.0]),
            (Head, [0.3, 0.0]),
            (Stopped, [0.0, 0.0]),
            (Head, [0.4, 0.4]),
            (Head, [0.0, 1.0]),
            (Ended, [0.0, 0.0]),
            (Ended, [0.0, 0.0]),
        ];

        for expect in expects {
            let frame = bub_frame_reader.next().unwrap().unwrap();
            assert_eq!(bub_frame_reader.metadata.bub_state, expect.0);
            assert_eq!(frame.0, expect.1);
        }
    }

    #[test]
    fn read_expr_frames() {
        let mut metadata = BubMetadata {
            spec_version: 0,
            bub_id: BubID::new(0),
            bub_version: 0,
            frames: 8,
            samples_per_sec: 96000.0,
            lpcm_kind: LpcmKind::F64LE,
            bub_sample_kind: BubSampleKind::default_expr(),
            name: String::from("Expression"),

            bub_state: BubState::Stopped,
            head_absolute_frame: 0,

            bub_functions: BubFns::new(),
            foot_absolute_frame_plus_one: 0,
            next_head_absolute_frame: Some(2),

            crc: crate::crc::CRC_32K_4_2,
        };
        let speakers_absolute_coord = vec![(0.0, 0.0, 0.0).into(), (0.0, 0.0, 1.0).into()];

        // Write Metadata and get CRC
        let mut skip: Vec<u8> = Vec::new();
        metadata.write(&mut skip).unwrap();

        let data: &[u8] = &[
            // Frame 1

            // Frame 2
            &14u16.to_le_bytes()[..],
            b"1 2 3 Z==1 0.1",
            &1u64.to_le_bytes(),
            &3u64.to_le_bytes(),
            // Expr
            &1u16.to_le_bytes(),
            b"1",
            &[17, 247, 225, 70], // crc
            // Frame 3

            // Frame 4
            &12u16.to_le_bytes()[..],
            b"1 2 3 Z==1 1",
            &2u64.to_le_bytes(),
            &3u64.to_le_bytes(),
            // Expr
            &3u16.to_le_bytes(),
            b"1/n",
            &[48, 94, 190, 151], // crc
            // Frame 5

            // Frame 6
            &11u16.to_le_bytes()[..],
            b"1 2 3 Z<1 n",
            &1u64.to_le_bytes(),
            &0u64.to_le_bytes(),
            // Expr
            &3u16.to_le_bytes(),
            b"0.1",
            &[248, 137, 58, 64], // crc

                                 // Frame 7

                                 // Frame 8
        ]
        .concat();
        let mut bub_frame_reader: BubFrameReader<&[u8], f32> =
            BubFrameReader::new(data, metadata, speakers_absolute_coord);

        let expects = vec![
            (Stopped, [0.0, 0.0]),
            (Head, [0.0, 0.1]),
            (Stopped, [0.0, 0.0]),
            (Head, [0.0, 1.0]),
            (Body, [0.0, 0.5]),
            (Head, [0.1, 0.0]),
            (Ended, [0.0, 0.0]),
            (Ended, [0.0, 0.0]),
        ];

        for expect in expects {
            let frame = bub_frame_reader.next().unwrap().unwrap();
            assert_eq!(bub_frame_reader.metadata.bub_state, expect.0);
            assert_eq!(frame.0, expect.1);
        }
    }
}
