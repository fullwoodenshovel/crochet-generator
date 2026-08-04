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

    /// Takes hashmap representation of isolines and the segment node1 - node2.
    /// current_isoline is used to filter for only intersections with the previous isoline.
    /// The data inside the return type Ok variant resembles all intersections found: ((i, j), (p1, k1), Option<(p2, k2)>).
    /// The k values are the ks of the points of intersection. k1 < k2 (unless k1 - k2 is on a boundary).
    /// p1 and p2 is the node segment that it intersects with. To calculate the exact position of intersection,
    /// you'd need to perform an intersection calculation on the segments.
    /// (p2, k2) is Some in most cases, but None if the intersection lies exactly on an edge. In this case,
    /// p1 is the point of intersection.
    /// # Panics
    /// This panics if self.info is None
    pub(super) fn find_intersections(&self, map: &IsolinesMap, node1: &Node, node2: &Node, intersecting_isoline: Option<usize>) -> Result<Vec<((usize, usize), (NodeOnEdge, usize), Option<(NodeOnEdge, usize)>)>> {
        let info = self.get_info_unwrapped();
        let isolines = &info.isolines;

        let mut faces = match node1.connectivity {
            Connectivity::OnVertex(v) => self.vertex_to_faces[v].clone(),
            Connectivity::OnEdge(a, b) => self.get_connected_faces(a, b),
        };
        let faces2 = match node2.connectivity {
            Connectivity::OnVertex(v) => self.vertex_to_faces[v].clone(),
            Connectivity::OnEdge(a, b) => self.get_connected_faces(a, b),
        };

        faces.retain(|face| faces2.contains(face));

        let result = match faces.len() {
            // At least one node was on edge. We consider isoline points on the face,
            // and circle around to see if the backtrack edge is in-between
            1 => {
                let face = faces.pop().unwrap();
                let edges = self.edges_on_face(face);
                let isoline_points: Vec<_> = edges.into_iter().flat_map(|[a, b]| {
                    let edge = OnEdge::new(a, b);
                    map.get(&edge).cloned().unwrap_or_default().iter().filter_map(|(pos, index)|
                        match intersecting_isoline {
                            Some(current_isoline) => if index.0 == current_isoline {
                                Some((NodeOnEdge { edge, pos: *pos }, *index))
                            } else {
                                None
                            },
                            None => Some((NodeOnEdge { edge, pos: *pos }, *index)),
                        }
                    ).collect::<Vec<_>>()
                }).collect();

                if isoline_points.is_empty() {
                    Vec::new()
                } else {
                    // Group together circles. Linear search is fine on isoline_points, as it will be very small.
                    let mut circles = Vec::with_capacity(1);
                    for (node, (i, j, k)) in isoline_points {
                        let vec = match circles.iter_mut().find(|(key, _value)| *key == (i, j)) {
                            Some(v) => &mut v.1,
                            None => {
                                circles.push(((i, j), Vec::with_capacity(1)));
                                &mut circles.last_mut().unwrap().1
                            }
                        };

                        let Err(index) = vec.binary_search_by_key(&k, |(_node, k)| *k) else {
                            return Err(failed_assert_internal::<11>(Some("Some point processed twice.".to_string())));
                        };

                        vec.insert(index, (node, k));
                    }

                    #[derive(Debug, Clone, Copy)]
                    enum Intersects {
                        Isoline,
                        Backtrack,
                        IsoBack,
                        BackIso,
                        #[allow(clippy::enum_variant_names)]
                        Intersects,
                        None,
                    }

                    impl Intersects {
                        fn new() -> Self {
                            Self::None
                        }

                        fn isoline(&mut self) {
                            *self = match self {
                                Self::Backtrack => Self::BackIso,
                                Self::IsoBack => Self::Intersects,
                                Self::None => Self::Isoline,
                                Self::Isoline => Self::None,
                                Self::BackIso => Self::Backtrack,
                                Self::Intersects => Self::Intersects,
                            }
                        }

                        fn backtrack(&mut self) {
                            *self = match self {
                                Self::Isoline => Self::IsoBack,
                                Self::BackIso => Self::Intersects,
                                Self::None => Self::Backtrack,
                                Self::Backtrack => Self::None,
                                Self::IsoBack => Self::Isoline,
                                Self::Intersects => Self::Intersects,
                            }
                        }

                        fn intersects(self) -> Result<bool> {
                            assert_internal::<1>(
                                // It can be Self::Isoline due to how the parallelism is implemented.
                                matches!(self, Self::None | Self::Intersects | Self::Isoline),
                                Some(format!("{self:?}"))
                            )?;
                            Ok(matches!(self, Self::Intersects))
                        }
                    }

                    #[derive(Debug)]
                    enum PointType {
                        Isoline(NodeOnEdge, usize),
                        Backtrack(Node)
                    }

                    impl PointType {
                        fn get_pos(&self) -> PVec3 {
                            match self {
                                PointType::Isoline(node_on_edge, _) => node_on_edge.pos,
                                PointType::Backtrack(node) => node.pos,
                            }
                        }
                    }

                    // Circle around our face.
                    let mut connections = Vec::new();
                    for ((i, j), mut points) in circles {
                        let mut ordered_points = Vec::new();
                        for [a, b] in self.edges_on_face(face) {
                            let a_pos = self.model.mesh.vertices[a];
                            let on_edge = OnEdge::new(a, b);
                            let mut points_on_edge: Vec<_> = points.iter().filter_map(|(node, k)| if node.edge == on_edge { Some(PointType::Isoline(*node, *k)) } else { None }).collect();
                            for node in [node1, node2] {
                                let on_edge = match node.connectivity {
                                    Connectivity::OnVertex(ap) => ap == a,
                                    Connectivity::OnEdge(ap, bp) => (ap == a && bp == b) || (ap == b && bp == a),
                                };

                                if on_edge {
                                    points_on_edge.push(PointType::Backtrack(*node));
                                }
                            }
                            points_on_edge.sort_by_cached_key(|point| (point.get_pos() - a_pos.into()).magnitude_squared().ord());
                            ordered_points.append(&mut points_on_edge);
                        }

                        assert_internal::<2>(
                            ordered_points.len() >= 4 && ordered_points.len().is_multiple_of(2),
                            Some(format!("{:?}", ordered_points))
                        )?;

                        // We have to be careful when sorting by k,
                        // because the cyclical nature messes things up.
                        points.sort_by_cached_key(|(_, k)| *k);
                        let mut tests: Vec<_> = points.iter().map(|(node, k)| (k, node, Intersects::new())).collect();
                        for point in ordered_points {
                            match point {
                                PointType::Isoline(_, k) => {
                                    let index = tests.binary_search_by_key(&k, |(k, _, _)| **k).unwrap(); // unnecessary flexing
                                    tests[index].2.isoline();
                                    if let Some((next_k, _, intersects)) = tests.get_mut(index + 1) {
                                        if **next_k == k + 1 {
                                            intersects.isoline()
                                        }
                                    } else if isolines[i][j].1.len() == k + 1  && *tests[0].0 == 0 {    
                                        tests[0].2.isoline();
                                    }
                                },
                                PointType::Backtrack(_) => {
                                    for (_, _, intersects) in &mut tests {
                                        intersects.backtrack();
                                    }
                                },
                            }
                        }

                        if tests.len() >= 2 {
                            for [a, b] in tests.array_windows().map(|[a, b]| [a, b]).chain(Some([tests.last().unwrap(), tests.first().unwrap()])) {
                                // We arent testing a.2.intersects() because of reasons. (It's magic :D )
                                if b.2.intersects()? && (a.0 + 1) % isolines[i][j].1.len() == *b.0 {
                                    connections.push(((i, j), (*a.1, *a.0), Some((*b.1, *b.0))))
                                }
                            }
                        }
                    }

                    connections
                }
            },

            // Both nodes were on the same edge.
            // We only need to consider isoline points on this edge.
            2 => {
                let (a, b) = match node1.connectivity {
                    Connectivity::OnEdge(a, b) => (a, b),
                    Connectivity::OnVertex(a) => match node2.connectivity {
                        Connectivity::OnVertex(b) => (a, b),
                        Connectivity::OnEdge(a, b) => (a, b),
                    },
                };

                let vertex_pos = self.model.mesh.vertices[a].into();
                let d1 = (node1.pos - vertex_pos).magnitude_squared();
                let d2 = (node2.pos - vertex_pos).magnitude_squared();
                let (max, min) = if d1 > d2 {
                    (d1, d2)
                } else {
                    (d2, d1)
                };

                let interval = min..=max;

                let edge = OnEdge::new(a, b);
                map.get(&edge).map(|nodes| nodes.iter().filter_map(|(pos, (i, j, k))| {
                    if interval.contains(&(*pos - vertex_pos).magnitude_squared()) && intersecting_isoline.map(|d| d == *i).unwrap_or(true) {
                        Some(((*i, *j), (NodeOnEdge { edge, pos: *pos }, *k), None))
                    } else {
                        None
                    }
                })).into_iter().flatten().collect::<Vec<((usize, usize), (NodeOnEdge, usize), Option<(NodeOnEdge, usize)>)>>()
            },
            v => return Err(failed_assert_internal::<12>(Some(format!("Unexpected number of matching faces: {v}"))))
        };
        Ok(result)
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