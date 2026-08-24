use three_d::Srgba;

use crate::viewer::DisplayCommand;
use crate::process::{Group, PVec3, Processor, Result};
use crate::process::intermediate::StitchPoint;
use crate::process::connect::{CircleTree, IsolinesMap};

#[derive(Clone, Copy)]
pub struct NodeOnFace {
    pub pos: PVec3,
    pub face: usize
}

impl From<StitchPoint> for NodeOnFace {
    fn from(value: StitchPoint) -> Self {
        Self {
            pos: value.pos,
            face: value.face
        }
    }
}

/// BEWARE:
/// spaced_points is undirectional. The only guaruntee from spaced_points
/// is that the point at index 0 is closest to the children and parent index 0.
pub struct ProcessedTree {
    pub spaced_points: Vec<NodeOnFace>,
    pub children: Vec<ProcessedTree>,
    // another field to represent where branches happen?
}

impl Processor {
    /// # Panics
    /// This panics is self.info is None
    pub(super) fn preprocess(&self, tree: &CircleTree, isolines_map: &IsolinesMap, scw: f32) -> Result<ProcessedTree> {
        self.preprocess_internal(tree, isolines_map, 1, scw)
    }

    fn preprocess_internal(&self, tree: &CircleTree, isolines_map: &IsolinesMap, row: usize, scw: f32) -> Result<ProcessedTree> {
        let start_point;
    
        let mut children = Vec::with_capacity(tree.children.len());
        for child in &tree.children {
            children.push(self.preprocess_internal(child, isolines_map, row + 1, scw)?);
        }

        if let Some(child) = children.first() {
            let node = child.spaced_points.first().expect("Each isoline should be at least 1.5sc in length.");
            let (pos, _, index) = self.get_rev_trav_intersection(*node, isolines_map, Some(row))?;
            start_point = StitchPoint { face: node.face, isoline_index: index.2, pos };
        } else {
            // distance can be replaced with an interpolation along the length of the circle in future.
            start_point = self.move_on_circle(&tree.circle, 0, tree.circle[0].pos, 0.0)?;
        };

        let stitches = (tree.circle_len / scw).round() as usize;
        let scw = tree.circle_len / stitches as f32;
        let mut spaced_points: Vec<NodeOnFace> = Vec::with_capacity(stitches);
        let mut curr_stitch = start_point;
        spaced_points.push(curr_stitch.into());
        for _ in 0..stitches-1 {
            curr_stitch = self.move_on_circle(&tree.circle, curr_stitch.isoline_index, curr_stitch.pos, scw)?;
            spaced_points.push(curr_stitch.into());
        }

        for [a, b] in spaced_points.array_windows().map(|[a, b]| [a, b]).chain(Some([spaced_points.last().unwrap(), spaced_points.first().unwrap()])) {
            self.sender.send(DisplayCommand::Edge {
                a: a.pos,
                b: b.pos,
                thickness: self.model.radius * 0.02,
                colour: Srgba::BLUE,
                depth: true,
                group: Group::StitchRow,
            }).unwrap();
            self.sender.send(DisplayCommand::Point {
                pos: a.pos,
                radius: self.model.radius * 0.025,
                colour: Srgba::RED,
                depth: true,
                group: Group::StitchRow
            }).unwrap();
        }

        Ok(ProcessedTree { spaced_points, children })
    }
}