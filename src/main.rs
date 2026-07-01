mod camera;
mod debug;
mod model;
mod viewer;
mod app;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use viewer::Viewer;

#[tokio::main()]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "model.stl".to_string());

    let (sender, receiver) = mpsc::unbounded_channel();
    let viewer = Viewer::new(&path, receiver)?;
    tokio::spawn(app::main(sender, Arc::clone(&viewer.model)));

    viewer.run()
}