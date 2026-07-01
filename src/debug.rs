use three_d::*;

const CHUNK_SIZE: usize = 1024;

/* ============================================================
   DEBUG RENDERER
============================================================ */

pub struct DebugRenderer {
    context: Context,

    points_occluded: Vec<PointChunk>,
    points_overlay: Vec<PointChunk>,

    lines_occluded: Vec<LineChunk>,
    lines_overlay: Vec<LineChunk>,
}

impl DebugRenderer {
    pub fn new(context: &Context) -> Self {
        Self {
            context: Clone::clone(context),
            points_occluded: Vec::new(),
            points_overlay: Vec::new(),
            lines_occluded: Vec::new(),
            lines_overlay: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.points_occluded.clear();
        self.points_overlay.clear();
        self.lines_occluded.clear();
        self.lines_overlay.clear();
    }

    /* ============================================================
       POINTS
    ============================================================ */

    pub fn point(&mut self, position: Vec3, radius: f32, color: Srgba, depth: bool) {
        let chunks = if depth {
            &mut self.points_occluded
        } else {
            &mut self.points_overlay
        };

        if chunks.is_empty() || chunks.last().unwrap().instances.len() >= CHUNK_SIZE {
            chunks.push(PointChunk::new(&self.context, depth));
        }

        let chunk = chunks.last_mut().unwrap();

        chunk.instances.push(PointInstance {
            position,
            radius,
            color,
        });

        chunk.dirty = true;
    }

    /* ============================================================
       LINES
    ============================================================ */

    pub fn edge(&mut self, a: Vec3, b: Vec3, thickness: f32, color: Srgba, depth: bool) {
        let chunks = if depth {
            &mut self.lines_occluded
        } else {
            &mut self.lines_overlay
        };

        if chunks.is_empty() || chunks.last().unwrap().instances.len() >= CHUNK_SIZE {
            chunks.push(LineChunk::new(&self.context, depth));
        }

        let chunk = chunks.last_mut().unwrap();

        chunk.instances.push(LineInstance {
            a,
            b,
            thickness,
            color,
        });

        chunk.dirty = true;
    }

    /* ============================================================
       GPU UPLOAD
    ============================================================ */

    pub fn upload(&mut self) {
        for c in &mut self.points_occluded {
            c.upload(&self.context);
        }
        for c in &mut self.points_overlay {
            c.upload(&self.context);
        }
        for c in &mut self.lines_occluded {
            c.upload(&self.context);
        }
        for c in &mut self.lines_overlay {
            c.upload(&self.context);
        }
    }

    /* ============================================================
       RENDER STREAMS
    ============================================================ */

    pub fn occluded(&self) -> impl Iterator<Item = &dyn Object> {
        self.points_occluded
            .iter()
            .map(|c| &c.mesh as &dyn Object)
            .chain(self.lines_occluded.iter().map(|c| &c.mesh as &dyn Object))
    }

    pub fn overlay(&self) -> impl Iterator<Item = &dyn Object> {
        self.points_overlay
            .iter()
            .map(|c| &c.mesh as &dyn Object)
            .chain(self.lines_overlay.iter().map(|c| &c.mesh as &dyn Object))
    }
}

/* ============================================================
   POINTS
============================================================ */

struct PointInstance {
    position: Vec3,
    radius: f32,
    color: Srgba,
}

struct PointChunk {
    instances: Vec<PointInstance>,
    mesh: Gm<InstancedMesh, ColorMaterial>,
    dirty: bool,
}

impl PointChunk {
    fn new(context: &Context, depth: bool) -> Self {
        let cpu = CpuMesh::sphere(8);

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu),
            ColorMaterial {
                color: Srgba::WHITE,
                render_states: RenderStates {
                    depth_test: if depth {
                        DepthTest::Less
                    } else {
                        DepthTest::Always
                    },
                    cull: Cull::None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        Self {
            instances: Vec::new(),
            mesh,
            dirty: true,
        }
    }

    fn upload(&mut self, _context: &Context) {
        if !self.dirty {
            return;
        }

        let mut transforms = Vec::with_capacity(self.instances.len());
        let mut colors = Vec::with_capacity(self.instances.len());

        for i in &self.instances {
            transforms.push(
                Mat4::from_translation(i.position)
                    * Mat4::from_scale(i.radius),
            );

            colors.push(i.color);
        }

        let instances = Instances {
            transformations: transforms,
            colors: Some(colors),
            texture_transformations: None,
        };

        self.mesh.geometry.set_instances(&instances);
        self.dirty = false;
    }
}

/* ============================================================
   LINES (FIXED + STABLE CYLINDER ORIENTATION)
============================================================ */

struct LineInstance {
    a: Vec3,
    b: Vec3,
    thickness: f32,
    color: Srgba,
}

struct LineChunk {
    instances: Vec<LineInstance>,
    mesh: Gm<InstancedMesh, ColorMaterial>,
    dirty: bool,
}

impl LineChunk {
    fn new(context: &Context, depth: bool) -> Self {
        let cpu = CpuMesh::cylinder(8);

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu),
            ColorMaterial {
                color: Srgba::WHITE,
                render_states: RenderStates {
                    depth_test: if depth {
                        DepthTest::Less
                    } else {
                        DepthTest::Always
                    },
                    cull: Cull::None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        Self {
            instances: Vec::new(),
            mesh,
            dirty: true,
        }
    }

    fn upload(&mut self, _context: &Context) {
        if !self.dirty {
            return;
        }

        let mut transforms = Vec::with_capacity(self.instances.len());
        let mut colors = Vec::with_capacity(self.instances.len());

        for i in &self.instances {
            let dir = i.b - i.a;
            let len = dir.magnitude();

            if len < 1e-6 {
                continue;
            }

            let dir_norm = dir / len;

            // three-d cylinder is Z-axis aligned – rotate Z to the edge direction,
            // using the horizontal perpendicular as the fallback to keep the
            // thickness directions horizontal (Y or X will align with horiz_perp).
            let rotation = Quaternion::from_arc(Vec3::unit_x(), dir_norm, None);

            // Scale: X and Y are the radius (thickness), Z is the length
            let transform =
                Mat4::from_translation(i.a)
                * Mat4::from(rotation)
                * Mat4::from_nonuniform_scale(
                    len,           // X → now aligned to the edge
                    i.thickness,   // Y
                    i.thickness,   // Z
                )
                ;
                
            transforms.push(transform);
            colors.push(i.color);
        }

        let instances = Instances {
            transformations: transforms,
            colors: Some(colors),
            texture_transformations: None,
        };

        self.mesh.geometry.set_instances(&instances);
        self.dirty = false;
    }
}