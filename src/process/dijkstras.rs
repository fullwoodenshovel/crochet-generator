use super::*;

impl Processor {
    pub fn dijkstras(&self, metadata: Metadata) -> Vec<(OrderedFloat<f32>, Node)> {
        let epsilon = metadata.stitch_size / 2.0;

        // Initialise the frontier with the points touching the same face
        let mut frontier = Frontier::new();
        for node in self.get_nodes_connected_to_face(metadata.seed_face, epsilon) {
            frontier.update(node, (metadata.seed_pos - node.pos).magnitude());
        }

        // Main Dijkstra's loop
        while let Some((node, geo_len)) = frontier.pop_store_smallest() {
            let nodes = self.get_connected_nodes(node.connectivity, epsilon);
            for updating_node in nodes {
                frontier.update(updating_node, geo_len + (updating_node.pos - node.pos).magnitude());
            }
        }

        frontier.get_output()
    }

    /// Find which nodes are connected to which other nodes, for use in Dijkstra's
    pub fn get_connected_nodes(&self, connectivity: Connectivity, epsilon: f32) -> Vec<Node> {
        // Find the faces a node is connected to.
        let faces = match connectivity {
            Connectivity::OnVertex(index) => self.vertex_to_faces[index].clone(),
            Connectivity::OnEdge(v1, v2) => self.get_connected_faces(v1, v2),
        };

        // Find the verticies that lie on these faces
        let verticies: Vec<_> = faces.into_iter().map(|face_index| self.model.mesh.faces[face_index].vertices).collect();

        // Use the verticies in order to find the vertex-pairs describing the edges of the face
        let edges: Vec<_> = if let Connectivity::OnEdge(exempt1, exempt2) = connectivity {
            verticies.iter().flat_map(|verticies|
                if verticies.iter().map(|v| (*v == exempt1 || *v == exempt2) as u8).sum::<u8>() < 2 {
                    let [v1, v2, v3] = verticies;
                    vec![(v1, v2), (v2, v3), (v3, v1)]
                } else {
                    // Ignore this edge if our node is on the same edge
                    vec![]
                }
            ).collect()
        } else {
            verticies.iter().flat_map(|[v1, v2, v3]| [(v1, v2), (v2, v3), (v3, v1)]).collect()
        };

        // Use these vertex-pair edges in order to find the spaced points along this edge
        let spaced_points: Vec<_> = edges.into_iter().flat_map(|(a, b)| {
            let pos_a = self.model.mesh.vertices[*a];
            let pos_b = self.model.mesh.vertices[*b];
            let posses = spaced_points(pos_a, pos_b, epsilon);
            posses.into_iter().map(|pos| Node { connectivity: Connectivity::on_edge(*a, *b), pos })
        }).collect();

        // Remove duplicated verticies
        let mut verticies = remove_duplicates(verticies.into_flattened());
        if let Connectivity::OnVertex(vertex) = connectivity {
            // If our node is on a vertex, remove that from the list we will return
            let index = verticies.iter().position(|v| *v == vertex).expect("The vertex we are finding connections to is connected to the face its connected to.");
            verticies.swap_remove(index);
        };

        let verticies = verticies.into_iter().map(|index|
            Node { connectivity: Connectivity::OnVertex(index), pos: self.model.mesh.vertices[index].into() }
        );
        
        let mut result = spaced_points;
        result.append(&mut verticies.collect());
        result
    }

    pub fn get_nodes_connected_to_face(&self, face_index: usize, epsilon: f32) -> Vec<Node> {
        // Find the verticies that lie on these faces
        let verticies = self.model.mesh.faces[face_index].vertices;

        // Use the verticies in order to find the vertex-pairs describing the edges of the face
        let v1 = verticies[0];
        let v2 = verticies[1];
        let v3 = verticies[2];
        let edges = [(v1, v2), (v2, v3), (v3, v1)];

        // Use these vertex-pair edges in order to find the spaced points along this edge
        let spaced_points: Vec<_> = edges.into_iter().flat_map(|(a, b)| {
            let pos_a = self.model.mesh.vertices[a];
            let pos_b = self.model.mesh.vertices[b];
            let posses = spaced_points(pos_a, pos_b, epsilon);
            posses.into_iter().map(move |pos| Node { connectivity: Connectivity::on_edge(a, b), pos })
        }).collect();

        // This function does not produce duplicates.


        let verticies = verticies.into_iter().map(|index|
            Node { connectivity: Connectivity::OnVertex(index), pos: self.model.mesh.vertices[index].into() }
        );
        
        let mut result = spaced_points;
        result.append(&mut verticies.collect());
        result
    }
}