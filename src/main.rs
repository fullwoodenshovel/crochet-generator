mod camera;
mod debug;
mod model;
mod viewer;

use anyhow::Result;
use viewer::Viewer;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "model.stl".to_string());

    let viewer = Viewer::new(&path)?;

    viewer.run()
}