use three_d::InnerSpace;
use anyhow::{Context, Result};
use stl_io::{IndexedMesh, Vector};
use std::{
    fs::File,
    io::BufReader,
};

use three_d::{
    CpuMesh,
    Indices,
    Positions,
    Vec3,
};

/// Convert STL coordinates (Z-up) into viewer coordinates (Y-up).
#[inline]
fn zup_to_yup(v: Vector<f32>) -> Vector<f32> {
    Vector::new([
        v[0],
        v[2],
        -v[1],
    ])
}

pub struct Model {
    /// This is the mesh all of your geometry algorithms should use.
    pub mesh: IndexedMesh,

    pub centre: Vec3,
    pub radius: f32,
}

impl Model {
    pub fn load(path: &str) -> Result<Self> {
        let file = File::open(path)
            .with_context(|| format!("Couldn't open STL file '{}'", path))?;

        let mut mesh = stl_io::read_stl(&mut BufReader::new(file))
            .context("Couldn't parse STL")?;

        // Convert to Y-up once so the rest of the program
        // never has to care about coordinate systems.
        for v in &mut mesh.vertices {
            *v = zup_to_yup(*v);
        }

        for face in &mut mesh.faces {
            face.normal = zup_to_yup(face.normal);
        }

        let (centre, radius) = bounding_sphere(&mesh);

        println!(
            "Loaded {} faces, {} vertices",
            mesh.faces.len(),
            mesh.vertices.len()
        );

        Ok(Self {
            mesh,
            centre,
            radius,
        })
    }

    /// Convert our IndexedMesh into a three-d CpuMesh.
    ///
    /// This is only used by the renderer.
    /// Your geometry algorithms should ignore it completely.
    pub fn cpu_mesh(&self) -> CpuMesh {
        let positions: Vec<Vec3> = self.mesh
            .vertices
            .iter()
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect();

        let mut indices = Vec::<u32>::with_capacity(
            self.mesh.faces.len() * 3
        );

        for face in &self.mesh.faces {
            indices.push(face.vertices[0] as u32);
            indices.push(face.vertices[1] as u32);
            indices.push(face.vertices[2] as u32);
        }

        let mut mesh = CpuMesh {
            positions: Positions::F32(positions),
            indices: Indices::U32(indices),

            normals: None,
            tangents: None,
            uvs: None,
            colors: None,
        };

        // Compute smooth normals.
        // Later we can replace this with flat shading if desired.
        mesh.compute_normals();

        mesh
    }
}

/// Recompute the bounding sphere after editing vertices.
pub fn bounding_sphere(mesh: &IndexedMesh) -> (Vec3, f32) {
    let mut lo = Vec3::new(
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
    );

    let mut hi = Vec3::new(
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );

    for v in &mesh.vertices {
        let p = Vec3::new(
            v[0],
            v[1],
            v[2],
        );

        lo.x = lo.x.min(p.x);
        lo.y = lo.y.min(p.y);
        lo.z = lo.z.min(p.z);

        hi.x = hi.x.max(p.x);
        hi.y = hi.y.max(p.y);
        hi.z = hi.z.max(p.z);
    }

    let centre = (lo + hi) * 0.5;
    let radius = (hi - lo).magnitude() * 0.5;

    (centre, radius)
}