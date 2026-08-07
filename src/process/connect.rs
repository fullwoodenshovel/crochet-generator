// This file is not AI generated

use std::matches;

use three_d::egui::emath::Float;

use crate::process::isolines::OnEdge;

use super::*;

pub type IsolinesMap = HashMap<OnEdge, Vec<(PVec3, (usize, usize, usize))>>;

pub struct CircleTree {
    pub circle: Vec<NodeOnEdge>,
    pub circle_len: f32,
    pub children: Vec<CircleTree>,
}

#[derive(Debug, Clone, Copy)]
struct IndexedCircle {
    circle: (usize, usize),
    prev: Option<(usize, usize)>
}

#[derive(Clone)]
struct IndexedCircleTree {
    circle: (usize, usize),
    children: Vec<IndexedCircleTree>
}

impl IndexedCircleTree {
    fn into_circle_tree(self, isolines: &[Vec<(f32, Vec<NodeOnEdge>)>]) -> CircleTree {
        let (circle_len, circle) = isolines[self.circle.0][self.circle.1].clone();
        CircleTree {
            circle,
            circle_len,
            children: self.children.into_iter().map(|child| child.into_circle_tree(isolines)).collect(),
        }
    }
}

impl Processor {
    /// This panics if self.info is None
    pub(super) fn connect(&self, map: &IsolinesMap, isolines: &[Vec<(f32, Vec<NodeOnEdge>)>]) -> Result<CircleTree> {
        self.sender.send(DisplayCommand::Clear(Group::IsolineConnectors)).unwrap();
        self.sender.send(DisplayCommand::Clear(Group::IsolinePoints)).unwrap();
        let info = self.get_info_unwrapped();
        let nodes = &info.nodes;
        let epsilon = &info.epsilon;

        let mut indexed_circle_trees = vec![IndexedCircle { circle: (0, 0), prev: None }];

        assert(
            isolines[0].len() == 1,
            "A split occurs before the first row",
            "Chose a different seed point, a different STL file, or decrease relative stitch size (by increasing diameter or decreasing hook size).",
            ErrorFault::User
        )?;

        for (i, isoline) in isolines[1..].iter().enumerate() {
            'circle: for (j, circle) in isoline.iter().enumerate() {

                // Optimisation. This checks if there is only one circle in the previous
                // isoline, and connects to that directly instead of backtracking.
                // This also means that it is less likely to error.
                if isolines[i].len() == 1 {
                    indexed_circle_trees.push(IndexedCircle { circle: (i+1, j), prev: Some((i, 0)) });
                    continue 'circle;
                }

                let circle_node = circle.1.first().unwrap();
                let OnEdge { a, b } = circle_node.edge;
                let pos = circle_node.pos;
                let mut closest = &self.nodes_on_edge(a, b, *epsilon).into_iter().min_by_key(|node| (node.pos - pos).magnitude_squared().ord()).unwrap();

                while let (_geo_len, Some(node)) = nodes.get(closest).unwrap() {
                    let mut intersections = self.find_intersections(map, node, closest, Some(i))?;

                    if let Some((circle, _, _)) = intersections.first()
                    && let Some((problem, _, _)) = intersections.iter().find(|v| v.0 != *circle)
                    {
                        return Err(failed_assert_internal::<10>(Some(format!("Circle backtrack intersected more than 1 previous circles simultaneously: {circle:?} and {problem:?}"))))
                    }

                    self.sender.send(DisplayCommand::Edge {
                        a: node.pos.into(),
                        b: closest.pos.into(),
                        thickness: self.model.radius * 0.02,
                        colour: Srgba::GREEN,
                        depth: true,
                        group: Group::Backtrack
                    }).unwrap();

                    if let Some((circle, (p1, _), poss)) = intersections.pop() {
                        match poss {
                            Some((p2, _)) => self.sender.send(DisplayCommand::Edge {
                                a: p1.pos.into(),
                                b: p2.pos.into(),
                                thickness: self.model.radius * 0.03,
                                colour: Srgba::RED,
                                depth: true,
                                group: Group::Backtrack
                            }).unwrap(),
                            None => self.sender.send(DisplayCommand::Point {
                                pos: p1.pos.into(),
                                radius: self.model.radius * 0.03,
                                colour: Srgba::RED,
                                depth: true,
                                group: Group::Backtrack
                            }).unwrap(),
                        }
                        

                        indexed_circle_trees.push(IndexedCircle { circle: (i+1, j), prev: Some(circle)});
                        continue 'circle;
                    }
                    closest = node
                }

                // The only reason it would get to this point is if it is the first isoline.
                // The first isoline has already been removed at the start.
                // The issue is, it can also get to this point if the mesh isn't dense enough near the seed point.
                return Err(Error {
                    issue: "Mesh wasn't dense enough near seed point.".to_string(),
                    fault: ErrorFault::File,
                    solution: "Chose a denser STL file, a different seed point, or increase relative stitch size (by decreasing diameter or increasing hook size).",
                });
            }
        }

        Ok(connect_tree(indexed_circle_trees).into_circle_tree(isolines))
    }

    pub(super) fn get_isoline_map(&self, isolines: &[Vec<(f32, Vec<NodeOnEdge>)>]) -> IsolinesMap {
        let mut map = HashMap::new();
        for (i, isoline) in isolines.iter().enumerate() {
            for (j, (_circle_len, circle)) in isoline.iter().enumerate() {
                for (k, node) in circle.iter().enumerate() {
                    map.entry(node.edge).or_insert_with(|| Vec::with_capacity(1)).push((node.pos, (i, j, k)));
                }
            }
        };
        map
    }
}


/// This function takes in all the items in the tree, and then puts them into the tree structure.
/// This could probably be optimised a LOT, but might not be necessary if it is fast enough.
fn connect_tree(items: Vec<IndexedCircle>) -> IndexedCircleTree {
    let mut layers = Vec::new();
    for item in items {
        while layers.len() <= item.circle.0 {
            layers.push(Vec::new());
        }
        layers[item.circle.0].push(item);
    }
    let layers = layers;

    let mut tree_layers: Vec<Vec<IndexedCircleTree>> = layers.clone().into_iter().map(|layer|
        layer.into_iter().map(|circle|
            IndexedCircleTree { circle: circle.circle, children: Vec::new() }
        ).collect()
    ).collect();

    for (i, layer) in layers.into_iter().enumerate().rev() {
        for (j, circle) in layer.into_iter().enumerate() {
            if i == 0 { continue; }
            let (s1, s2) = tree_layers.split_at_mut(i);
            let below_layer = &mut s2[0];
            let above_layer = &mut s1[i-1];
            for possible_parent in above_layer {
                if possible_parent.circle == circle.prev.unwrap() {
                    possible_parent.children.push(below_layer[j].clone());
                }
            }
        }
    }

    tree_layers[0][0].clone()
}