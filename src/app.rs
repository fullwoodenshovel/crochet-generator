use std::{sync::Arc, time::Duration};

use three_d::Srgba;
use tokio::sync::mpsc;

use crate::{model::Model, viewer::Command};

pub async fn main(sender: mpsc::UnboundedSender<Command>, model: Arc<Model>) {
    let mut i = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let pos = model.mesh.vertices[i];
        i += 1;
        sender.send(Command::Point { pos, radius: model.radius * 0.02, colour: Srgba::GREEN, depth: false }).unwrap();
    }
}