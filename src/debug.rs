use three_d::*;

/// One frame's worth of debug primitives. Clear and re-populate it each
/// frame, then call `build()` to get renderable objects.
pub struct DebugRenderer {
    points: Vec<(Vec3, f32, Srgba, bool)>,
    edges: Vec<(Vec3, Vec3, f32, Srgba, bool)>,
    faces: Vec<(Vec<Vec3>, Srgba, bool)>,
}

pub struct BuildResult(pub Vec<Gm<Mesh, ColorMaterial>>, pub Vec<Gm<Mesh, ColorMaterial>>);

impl DebugRenderer {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.edges.clear();
        self.faces.clear();
    }

    /// depth_test = true: occluded by the model normally.
    /// depth_test = false: always drawn on top (X-ray style).
    pub fn point(&mut self, position: Vec3, radius: f32, color: Srgba, depth_test: bool) {
        self.points.push((position, radius, color, depth_test));
    }

    pub fn edge(&mut self, a: Vec3, b: Vec3, thickness: f32, color: Srgba, depth_test: bool) {
        self.edges.push((a, b, thickness, color, depth_test));
    }

    /// 3+ verts, fan-triangulated from verts[0]. No culling, so order
    /// doesn't matter, but it must be convex (or close enough).
    pub fn face(&mut self, verts: &[Vec3], color: Srgba, depth_test: bool) {
        if verts.len() >= 3 {
            self.faces.push((verts.to_vec(), color, depth_test));
        }
    }

    /// Build this frame's debug geometry as renderable objects, split into
    /// two groups:
    ///
    ///   .0 — "occluded" objects: render these in the SAME pass as the
    ///        model, with normal depth testing.
    ///   .1 — "overlay" objects: render these in a SECOND pass, after
    ///        clearing only the depth buffer (see `viewer.rs`). This is
    ///        what actually guarantees "always on top of everything",
    ///        rather than relying on a per-material DepthTest::Always
    ///        flag fighting the renderer's own opaque/transparent split.
    pub fn build(
        &self,
        context: &Context,
    ) -> BuildResult {
        let mut occluded = Vec::new();
        let mut overlay = Vec::new();

        for &(pos, radius, color, depth_test) in &self.points {
            let mut cpu_mesh = CpuMesh::sphere(8);
            cpu_mesh
                .transform(Mat4::from_translation(pos) * Mat4::from_scale(radius))
                .unwrap();

            let gm = Gm::new(
                Mesh::new(context, &cpu_mesh),
                ColorMaterial {
                    color,
                    render_states: render_states(),
                    ..Default::default()
                },
            );
            push(&mut occluded, &mut overlay, gm, depth_test);
        }

        for &(a, b, thickness, color, depth_test) in &self.edges {
            let cpu_mesh = stretched_cylinder(a, b, thickness);

            let gm = Gm::new(
                Mesh::new(context, &cpu_mesh),
                ColorMaterial {
                    color,
                    render_states: render_states(),
                    ..Default::default()
                },
            );
            push(&mut occluded, &mut overlay, gm, depth_test);
        }

        for (verts, color, depth_test) in &self.faces {
            let cpu_mesh = fan_mesh(verts);

            let gm = Gm::new(
                Mesh::new(context, &cpu_mesh),
                ColorMaterial {
                    color: *color,
                    render_states: render_states(),
                    ..Default::default()
                },
            );
            push(&mut occluded, &mut overlay, gm, *depth_test);
        }

        BuildResult(occluded, overlay)
    }
}

fn push(
    occluded: &mut Vec<Gm<Mesh, ColorMaterial>>,
    overlay: &mut Vec<Gm<Mesh, ColorMaterial>>,
    gm: Gm<Mesh, ColorMaterial>,
    depth_test: bool,
) {
    if depth_test {
        occluded.push(gm);
    } else {
        overlay.push(gm);
    }
}

/// Both groups use plain `Less` depth testing. The "always on top" effect
/// for overlay objects comes from clearing the depth buffer before their
/// pass in `viewer.rs`, NOT from a special depth-test mode — this also
/// means overlay objects still correctly occlude each other if you draw
/// more than one.
fn render_states() -> RenderStates {
    RenderStates {
        depth_test: DepthTest::Less,
        cull: Cull::None,
        ..Default::default()
    }
}

/// Build a CpuMesh::cylinder(...) (unit cylinder along +x, radius 1,
/// length 1) scaled/rotated/translated to run from `a` to `b`.
fn stretched_cylinder(a: Vec3, b: Vec3, radius: f32) -> CpuMesh {
    let mut cpu_mesh = CpuMesh::cylinder(8);

    let dir = b - a;
    let length = dir.magnitude();
    let dir_norm = if length > 1e-6 { dir / length } else { Vec3::unit_x() };

    let x_axis = Vec3::unit_x();
    let rotation = if dir_norm.dot(x_axis) > 0.999_99 {
        Quaternion::from_angle_y(Rad(0.0))
    } else if dir_norm.dot(x_axis) < -0.999_99 {
        Quaternion::from_angle_z(Rad(std::f32::consts::PI))
    } else {
        let axis = x_axis.cross(dir_norm).normalize();
        let angle = x_axis.dot(dir_norm).acos();
        Quaternion::from_axis_angle(axis, Rad(angle))
    };

    let transform = Mat4::from_translation(a)
        * Mat4::from(rotation)
        * Mat4::from_nonuniform_scale(length, radius, radius);

    cpu_mesh.transform(transform).unwrap();
    cpu_mesh
}

/// Fan-triangulate a convex polygon (vertex 0 shared by every triangle).
fn fan_mesh(verts: &[Vec3]) -> CpuMesh {
    let n = verts.len() as u32;
    let indices: Vec<u32> = (1..n - 1).flat_map(|i| [0u32, i, i + 1]).collect();

    let mut cpu_mesh = CpuMesh {
        positions: Positions::F32(verts.to_vec()),
        indices: Indices::U32(indices),
        ..Default::default()
    };
    cpu_mesh.compute_normals();
    cpu_mesh
}