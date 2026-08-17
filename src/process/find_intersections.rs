// This file is partially AI generated (one function)
// All AI used in this file is documented.

use crate::process::{Connectivity, Node, Processor, Result, assert_internal, failed_assert_internal};
use crate::process::connect::IsolinesMap;
use crate::process::process_vec3::PVec3;
use crate::process::isolines::{NodeOnEdge, OnEdge};
use three_d::egui::emath::Float;

// This function is AI generated
fn segment_intersect(n: PVec3, a: PVec3, b: PVec3, c: PVec3, d: PVec3) -> Option<PVec3> {
    let r = b - a;
    let s = d - c;

    let denom = r.cross(s).dot(n);
    if denom.abs() < 1e-6 {
        return None; // parallel (or collinear) - no unique intersection
    }

    let t = (c - a).cross(s).dot(n) / denom;
    let u = (c - a).cross(r).dot(n) / denom;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(a + r * t)
    } else {
        None
    }
}

impl Processor {
    /// Takes hashmap representation of isolines and the segment node - (pos face).
    /// current_isoline is used to filter for only intersections with the previous isoline.
    /// The data inside the return type Ok variant resembles all intersections found: ((i, j), pos, [(p, k); 2]).
    /// The k values are the ks of the points of intersection. k1 < k2 (unless k1 - k2 is on a boundary).
    /// p1 and p2 is the node segment that it intersects with. pos is the position of intersection.
    /// # Panics
    /// This panics if self.info is None
    // This is less optimised because we can't do our circling around trick anymore, we need to actually check if each line intersects.
    // This also means, however, that this code is less prone to bugs.
    pub(super) fn find_face_intersections(&self, map: &IsolinesMap, node: &Node, face: usize, pos: PVec3, intersecting_isoline: Option<usize>) -> Result<Vec<((usize, usize), PVec3, [(NodeOnEdge, usize); 2])>> {
        let info = self.get_info_unwrapped();
        let isolines = &info.isolines;

        let face_normal = self.model.mesh.faces[face].normal.into();
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
            Ok(Vec::new())
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
                    return Err(failed_assert_internal::<15>(Some("Some point processed twice.".to_string())));
                };

                vec.insert(index, (node, k));
            }

            let mut connections = Vec::new();

            for ((i, j), circle) in circles {
                for [(n1, k1), (n2, k2)] in circle.array_windows().map(|[a, b]| [a, b]).chain(Some([circle.last().unwrap(), circle.first().unwrap()])) {
                    if (k1 + 1) % isolines[i][j].1.len() == *k2 && let Some(intersection) = segment_intersect(face_normal, n1.pos, n2.pos, pos, node.pos) {
                        connections.push(((i, j), intersection, [(*n1, *k1), (*n2, *k2)]));
                    } else if let Connectivity::OnEdge(a, b) = node.connectivity && n1.edge == (OnEdge{a, b}) && n2.edge == (OnEdge{a, b}) && // If the node lies on the same edge
                    let distance = (n1.pos - n2.pos).magnitude_squared() && // And it is closer to both nodes
                    (n1.pos - node.pos).magnitude_squared() < distance && (n2.pos - node.pos).magnitude_squared() < distance {
                        // This secondary check is needed due to floating point imprecision.
                        // Without it, intersections could be missed
                        connections.push(((i, j), node.pos, [(*n1, *k1), (*n2, *k2)]));
                    }
                }
            }

            Ok(connections)
        }
    }

    /// Takes hashmap representation of isolines and the segment (pos1 face) - (pos2 face).
    /// current_isoline is used to filter for only intersections with the previous isoline.
    /// The data inside the return type Ok variant resembles all intersections found: ((i, j), pos, [(p, k); 2]).
    /// The k values are the ks of the points of intersection. k1 < k2 (unless k1 - k2 is on a boundary).
    /// p1 and p2 is the node segment that it intersects with. pos is the position of intersection.
    /// # Panics
    /// This panics if self.info is None
    // This just calls self.find_face_intersections with a specific, invalid node. This is just a mathematical
    // trick that works because we are never checking the connectivity of the node in self.find_face_intersections,
    // if that connectivity is OnVertex.
    pub(super) fn find_double_face_intersections(&self, map: &IsolinesMap, face: usize, pos1: PVec3, pos2: PVec3, intersecting_isoline: Option<usize>) -> Result<Vec<((usize, usize), PVec3, [(NodeOnEdge, usize); 2])>> {
        self.find_face_intersections(map, &Node { connectivity: Connectivity::OnVertex(0), pos: pos1 }, face, pos2, intersecting_isoline)
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