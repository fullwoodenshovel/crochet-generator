use three_d::*;

pub struct DebugRenderer {
    points: PointCloud,
    lines: LineCloud,
}

impl DebugRenderer {
    pub fn new(context: &Context) -> Self {
        Self {
            points: PointCloud::new(context),
            lines: LineCloud::new(context),
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.lines.clear();
    }

    pub fn point(&mut self, position: Vec3, radius: f32, color: Srgba, _depth_test: bool) {
        self.points.push(position, radius, color);
    }

    pub fn edge(&mut self, a: Vec3, b: Vec3, thickness: f32, color: Srgba, _depth_test: bool) {
        self.lines.push(a, b, thickness, color);
    }

    pub fn upload(&mut self) {
        self.points.upload();
        self.lines.upload();
    }

    pub fn occluded(&self) -> impl Iterator<Item = &dyn Object> {
        std::iter::once(self.points.as_object())
            .chain(std::iter::once(self.lines.as_object()))
    }
}

/* ============================================================
   POINT CLOUD (Instanced spheres)
============================================================ */

struct PointInstance {
    transform: Mat4,
    color: Srgba,
}

pub struct PointCloud {
    instances: Vec<PointInstance>,
    dirty: bool,
    mesh: Gm<InstancedMesh, ColorMaterial>,
}

impl PointCloud {
    pub fn new(context: &Context) -> Self {
        let cpu = CpuMesh::sphere(8);

        let material = ColorMaterial {
            color: Srgba::WHITE,
            render_states: render_states(),
            ..Default::default()
        };

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu),
            material,
        );

        Self {
            instances: Vec::new(),
            dirty: false,
            mesh,
        }
    }

    pub fn push(&mut self, position: Vec3, radius: f32, color: Srgba) {
        self.instances.push(PointInstance {
            transform: Mat4::from_translation(position)
                * Mat4::from_scale(radius),
            color,
        });

        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
    }

    pub fn upload(&mut self) {
        if !self.dirty {
            return;
        }

        let instances = Instances {
            transformations: self.instances.iter().map(|i| i.transform).collect(),
            colors: Some(self.instances.iter().map(|i| i.color).collect()),
            texture_transformations: None,
        };

        self.mesh.geometry.set_instances(&instances);
        self.dirty = false;
    }

    fn as_object(&self) -> &dyn Object {
        &self.mesh
    }
}

/* ============================================================
   LINE CLOUD (Instanced cylinders)
============================================================ */

struct LineInstance {
    transform: Mat4,
    color: Srgba,
}

pub struct LineCloud {
    instances: Vec<LineInstance>,
    dirty: bool,
    mesh: Gm<InstancedMesh, ColorMaterial>,
}

impl LineCloud {
    pub fn new(context: &Context) -> Self {
        let cpu = CpuMesh::cylinder(8);

        let material = ColorMaterial {
            color: Srgba::WHITE,
            render_states: render_states(),
            ..Default::default()
        };

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu),
            material,
        );

        Self {
            instances: Vec::new(),
            dirty: false,
            mesh,
        }
    }

    pub fn push(&mut self, a: Vec3, b: Vec3, thickness: f32, color: Srgba) {
        let dir = b - a;
        let len = dir.magnitude();

        if len < 1e-6 {
            return;
        }

        let dir = dir / len;
        let x_axis = Vec3::unit_x();

        let rotation = if dir.dot(x_axis) > 0.999_99 {
            Quaternion::new(1.0, 0.0, 0.0, 0.0)
        } else if dir.dot(x_axis) < -0.999_99 {
            Quaternion::from_angle_z(Rad(std::f32::consts::PI))
        } else {
            let axis = x_axis.cross(dir).normalize();
            let angle = x_axis.dot(dir).acos();
            Quaternion::from_axis_angle(axis, Rad(angle))
        };

        let transform =
            Mat4::from_translation(a)
            * Mat4::from(rotation)
            * Mat4::from_nonuniform_scale(len, thickness, thickness);

        self.instances.push(LineInstance {
            transform,
            color,
        });

        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
    }

    pub fn upload(&mut self) {
        if !self.dirty {
            return;
        }

        let instances = Instances {
            transformations: self.instances.iter().map(|i| i.transform).collect(),
            colors: Some(self.instances.iter().map(|i| i.color).collect()),
            texture_transformations: None,
        };

        self.mesh.geometry.set_instances(&instances);
        self.dirty = false;
    }

    fn as_object(&self) -> &dyn Object {
        &self.mesh
    }
}

/* ============================================================
   helper
============================================================ */

fn render_states() -> RenderStates {
    RenderStates {
        depth_test: DepthTest::Less,
        cull: Cull::None,
        ..Default::default()
    }
}