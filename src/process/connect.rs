use super::*;

pub enum StitchCommand {
    Single,
    Inc(usize),
    Dec(usize),
}

impl Processor {
    pub fn connect(&self, isolines: Vec<Vec<(f32, Vec<NodeOnEdge>)>>, furthest_point: Node) -> Vec<StitchCommand> {

        todo!()
    }
}