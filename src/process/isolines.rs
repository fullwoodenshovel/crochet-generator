use std::collections::HashMap;

use super::*;

impl Processor {
    /// The output of this function is a list of all isolines, connected in a circle, e.g.
    /// ```
    /// [ // <- Outer list
    ///     [ // <- Isolines at geodesic length w
    ///         [Node1, Node2, Node3], // <- Circle 1
    ///         [Node4, Node5, Node6] // <- Circle 2
    ///         // (the fact that there are multiple lists in this list means the isoline is split.)
    ///     ], 
    ///     [ // <- Isolines at geodesic length w * 2
    ///         [Node7, Node8, Node9] // <- Circle 3
    ///     ]
    /// ]
    /// ```
    pub async fn isolines(&mut self, nodes: Vec<(OrderedFloat<f32>, Node)>, stitch_size: f32, epsilon: f32) -> Vec<Vec<Vec<Node>>> {
        // This could be optimised by moving this type conversion into dijkstras.rs immediately.
        // However, we need to store the longest length alongside this data, if this optimisation is used.
        let len = nodes[nodes.len() - 1].0.0;
        let old_ss = stitch_size;
        let total_lines = (len / old_ss).round();
        let stitch_size = len / total_lines;
        let div_1_stitch_size = 1.0 / stitch_size;
        println!("Stitch size off by {:.2}%", 100.0 * (1.0 - stitch_size / old_ss));
        let mut map = HashMap::new();
        for (geo_len, node) in nodes {
            map.entry(node).insert_entry(geo_len);
        }

        let mut isoline_points = vec![Vec::new(); total_lines as usize + 1];
        for face in &self.model.mesh.faces {
            let v = face.vertices;
            for [a, b] in [[v[0], v[1]], [v[1], v[2]], [v[2], v[0]]] {
            let (a, b) = if a > b {
                (a, b)
            } else {
                (b, a)
            };
                let mut nodes = Vec::with_capacity(1);
                let node_a = self.node_from_vertex(a);
                let node_b = self.node_from_vertex(b);
                nodes.append(&mut self.spaced_points(a, b, epsilon).into_iter().map(|pos| {
                    let node = self.node_from_spacing(a, b, pos);
                    let geo_len = map.get(&node).unwrap();
                    (pos, geo_len.0)
                }).collect());
                nodes.push((node_a.pos, map.get(&node_a).unwrap().0));

                let mut prev_line = map.get(&node_b).unwrap().0 * div_1_stitch_size;
                let mut prev_line_floor = (map.get(&node_b).unwrap().0 * div_1_stitch_size).floor();
                let mut prev_pos = node_b.pos;

                for (pos, len) in nodes {
                    let line = len * div_1_stitch_size;
                    let line_floor = line.floor();
                    if line_floor != prev_line_floor {
                        if (line_floor - prev_line_floor).abs() != 1.0 {
                            println!("ERR");
                            continue;
                        }
                        let mid = line_floor.max(prev_line_floor);
                        let ib = (mid - prev_line) / (line - prev_line);
                        let ia = 1.0 - ib;
                        isoline_points[mid as usize].push(prev_pos * ia + pos * ib);
                        self.sender.send(DisplayCommand::Point { pos: (prev_pos * ia + pos * ib).into(), radius: self.model.radius * 0.025, colour: Srgba::WHITE, depth: true, temp: true }).unwrap();
                        if !(0.0..=1.0).contains(&ia) {
                            println!("out rage");
                        } else {
                            println!("in range");
                        }
                        // println!("mid: {mid}; line: {line}; prev_line: {prev_line}");
                        // println!("ia: {ia}; pos: {:?}", prev_pos * ia + pos * ib);
                    }
                    // self.receiver.recv().await;
                    // self.sender.send(DisplayCommand::ClearTempPoints).unwrap();
                    prev_line = line;
                    prev_line_floor = line_floor;
                    prev_pos = pos;
                }
            }
        }

        todo!()
    }

    fn node_from_vertex(&self, index: usize) -> Node {
        Node { connectivity: Connectivity::OnVertex(index), pos: self.model.mesh.vertices[index].into() }
    }

    fn node_from_spacing(&self, a: usize, b: usize, pos: PVec3) -> Node {
        Node { connectivity: Connectivity::on_edge(a, b), pos }
    }
}


// I have two geodesic lengths, a and b. these are around a boundary mid.
// I want to linearly interpolate between the positions of a and b in order
// to estimate where the boundary point lies. I need to find an equation
// in terms of a, b, pos_a, pos_b and mid that gives me pos_mid.

// a = 6.2 = len_a / stitch_size
// b = 5.9 = len_b / stitch_size

// diff = b - a
// mid = 6.0
// (a - mid)/(a - b)

// (b - a)*I = mid

// a * IA + b * IB = mid
// IA + IB = 1
// solve for IA and IB:
//
// IA = 1 - IB
// a * (1 - IB) + b * IB = mid
// a - a*IB + b * IB = mid
// b * IB - a * IB + a = mid
// IB (b - a) + a = mid
// IB = (mid - a)/(b-a)
// IA = 1 - IB
//
// pos_mid = pos_a * IA + pos_b * IB