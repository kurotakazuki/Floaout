use crate::{Coord, Rgba};

// TODO: Add feilds like frame_span, vertex_spacing, colors, and so on.
// f32 -> f64 in the future
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OaoSpace {
    pub vertex_spacing: f32,
    pub start: f32,
    pub range: usize,
    pub vertices: Vec<Rgba>,
}

impl OaoSpace {
    pub const fn new() -> Self {
        Self {
            vertex_spacing: 0.2,
            start: -1.2,
            range: 13,
            vertices: Vec::new(),
        }
    }

    /// -1.0 ~ 1.0
    pub fn vertices_coord(&self) -> Vec<Coord<f32>> {
        let mut coords = Vec::new();
        let denominator = self.start.abs();
        for x in 0..self.range {
            let x = x as f32 * self.vertex_spacing + self.start;
            for y in 0..self.range {
                let y = y as f32 * self.vertex_spacing + self.start;
                for z in 0..self.range {
                    let z = z as f32 * self.vertex_spacing + self.start;
                    coords.push((x / denominator, y / denominator, z / denominator).into());
                }
            }
        }

        coords
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OaoSpaces {
    /// Number of frames between spaces
    pub frames_between_spaces: u64,
    pub vertex_spacing: f32,
    pub start: f32,
    pub range: usize,
    pub spaces: Vec<OaoSpace>,
}

impl OaoSpaces {
    pub const fn new() -> Self {
        Self {
            frames_between_spaces: 3200,
            // -1.2 ~ 1.2 (0.2 spacing)
            vertex_spacing: 0.2,
            start: -1.2,
            range: 13,
            spaces: Vec::new(),
        }
    }
}
