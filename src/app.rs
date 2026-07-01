use std::{sync::Arc, time::Duration};

use three_d::Srgba;
use tokio::sync::mpsc;

use crate::{model::Model, viewer::Command};

pub async fn main(sender: mpsc::UnboundedSender<Command>, model: Arc<Model>) {
    let mut vertex_to_faces: Vec<Vec<usize>> = std::iter::repeat_with(Vec::new).take(model.mesh.vertices.len()).collect();
    for (i, face) in model.mesh.faces.iter().enumerate() {
        for vertex in face.vertices {
            vertex_to_faces[vertex].push(i)
        }
    }
    let mut processor = Processor { sender, model, vertex_to_faces };

    processor.run().await;
}

struct Processor {
    sender: mpsc::UnboundedSender<Command>,
    model: Arc<Model>,
    vertex_to_faces: Vec<Vec<usize>>
}

impl Processor {
    async fn run(&mut self) {
        let mut i = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let p1 = self.model.mesh.vertices[i];
            let p2 = self.model.mesh.faces[self.vertex_to_faces[i][0]].vertices[0];
            self.sender.send(Command::Point { pos: p1, radius: self.model.radius * 0.02, colour: Srgba::GREEN, depth: false }).unwrap();
            self.sender.send(Command::Point { pos: self.model.mesh.vertices[p2], radius: self.model.radius * 0.02, colour: Srgba::GREEN, depth: false }).unwrap();
            for face in self.get_connected_faces(i, p2) {
                let face = self.model.mesh.faces[face].vertices.map(|i| self.model.mesh.vertices[i]);
                self.sender.send(Command::Face { a: face[0], b: face[1], c: face[2], colour: Srgba::RED, depth: true }).unwrap();
            }
            i += 5;
        }
    }

    fn get_connected_faces(&self, v1: usize, v2: usize) -> Vec<usize> {
        let mut faces1 = self.vertex_to_faces[v1].clone();
        let faces2 = &self.vertex_to_faces[v2];
        faces1.retain(|face| faces2.contains(face));
        faces1
    }
}
