use macroquad::prelude::*;
use stl_io::Vector;
use std::{env, fs::File, io::BufReader};

// ─── Coordinate convention ────────────────────────────────────────────────────

/// Re-orient from STL's Z-up convention to our Y-up world space.
///
///   STL (Z-up, right-hand)  →  viewer (Y-up, right-hand)
///   (x,  y,  z)             →  (x,  z, -y)
///
/// Apply to both vertex positions and face normals immediately after loading.
#[inline]
fn zup_to_yup(v: Vector<f32>) -> Vector<f32> {
    Vector::<f32>::new([v[0], v[2], -v[1]])
}
// ─── Orbit camera ─────────────────────────────────────────────────────────────

struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    /// Distance from the target point
    distance: f32,
    /// The world-space point the camera orbits around
    target: Vec3,
    prev_mouse: Option<Vec2>,
    /// The "reset" state to return to on R
    initial: (f32, f32, f32),
}

impl OrbitCamera {
    fn new(center: Vec3, radius: f32) -> Self {
        let yaw = 0.3_f32;
        let pitch = 0.4_f32;
        let distance = radius * 2.5;
        Self {
            yaw,
            pitch,
            distance,
            target: center,
            prev_mouse: None,
            initial: (yaw, pitch, distance),
        }
    }

    /// World-space eye position derived from the spherical coordinates.
    fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + self.distance * vec3(cp * sy, sp, cp * cy)
    }

    fn update(&mut self) {
        let (mx, my) = mouse_position();
        let m = vec2(mx, my);

        // Orbit on left-drag
        if is_mouse_button_down(MouseButton::Left) {
            if let Some(prev) = self.prev_mouse {
                let delta = m - prev;
                self.yaw -= delta.x * 0.005;
                self.pitch = (self.pitch + delta.y * 0.005).clamp(-1.55, 1.55);
            }
            self.prev_mouse = Some(m);
        } else {
            self.prev_mouse = None;
        }

        // Scroll to zoom
        let (_, scroll) = mouse_wheel();
        if scroll != 0.0 {
            self.distance = (self.distance * (1.0 - scroll * 0.1)).max(1e-4);
        }

        // Reset
        if is_key_pressed(KeyCode::R) {
            let (y, p, d) = self.initial;
            self.yaw = y;
            self.pitch = p;
            self.distance = d;
        }
    }
}

// ─── Debug 3-D primitives ─────────────────────────────────────────────────────
//
// Each function accepts a `depth_test` flag:
//   true  → occluded by mesh geometry in front of it (normal 3-D behaviour)
//   false → always drawn on top (X-ray / overlay style)
//
// Macroquad starts a fresh internal draw-call batch whenever the depth-test
// flag changes, so occluded and unoccluded calls can be freely interleaved
// within the same frame.  Every helper restores depth-test to `true` before
// returning so the surrounding render loop is unaffected.

fn set_depth_test(enable: bool) {
    // SAFETY: we only touch the depth-test flag; no memory invariants are broken.
    unsafe { get_internal_gl().quad_gl.depth_test(enable); }
}

/// Draw a point as a small sphere.
pub fn draw_debug_point(pos: Vec3, radius: f32, color: Color, depth_test: bool) {
    set_depth_test(depth_test);
    draw_sphere(pos, radius, None, color);
    set_depth_test(true);
}

/// Draw an edge (line segment) between two world-space points.
pub fn draw_debug_edge(a: Vec3, b: Vec3, color: Color, depth_test: bool) {
    set_depth_test(depth_test);
    draw_line_3d(a, b, color);
    set_depth_test(true);
}

/// Draw a filled convex polygon.
///
/// Pass 3 or more vertices in order (CW or CCW — no back-face culling is
/// applied).  The polygon is fan-triangulated from vertex 0, so concave
/// shapes will not render correctly; split them into convex pieces first.
pub fn draw_debug_face(verts: &[Vec3], color: Color, depth_test: bool) {
    if verts.len() < 3 {
        return;
    }
    set_depth_test(depth_test);

    let mesh_verts: Vec<Vertex> = verts
        .iter()
        .map(|&p| Vertex {
            position: p,
            uv: Vec2::ZERO,
            color: color.into(),
            normal: Vec4::ZERO,
        })
        .collect();

    // Fan triangulation: (0, i, i+1) for i in 1 .. n-2
    let n = verts.len() as u16;
    let indices: Vec<u16> = (1..n - 1).flat_map(|i| [0u16, i, i + 1]).collect();

    draw_mesh(&Mesh {
        vertices: mesh_verts,
        indices,
        texture: None,
    });

    set_depth_test(true);
}

// ─── Mesh conversion ──────────────────────────────────────────────────────────

/// Lambertian diffuse + ambient for a fixed directional light.
/// Returns a flat `Color` per face so shading is baked into vertex colours.
fn shade(normal: &[f32; 3]) -> Color {
    let light = vec3(1.0, 2.0, 3.0).normalize();
    let n = vec3(normal[0], normal[1], normal[2]);
    let len = n.length();
    let diffuse = if len > 1e-6 {
        (n / len).dot(light) / 2.0 + 0.5
    } else {
        0.0
    };
    let b = 0.18 + 0.82 * diffuse;
    Color::new(b * 0.75, b * 0.78, b * 0.90, 1.0)
}

/// Build batched macroquad `Mesh`es from an `IndexedMesh`.
///
/// Batching is required because `Mesh::indices` is `Vec<u16>`, capping at
/// 65 535 values (= 21 845 triangles per batch).  Each triangle gets its own
/// three vertices so per-face flat shading works without normal averaging.
fn build_batches(stl: &stl_io::IndexedMesh) -> Vec<Mesh> {
    const MAX_TRIS: usize = 1666; // floor(65535 / 3)

    stl.faces
        .chunks(MAX_TRIS)
        .map(|chunk| {
            let mut verts: Vec<Vertex> = Vec::with_capacity(chunk.len() * 3);
            let mut idxs: Vec<u16> = Vec::with_capacity(chunk.len() * 3);

            for (i, face) in chunk.iter().enumerate() {
                let color = shade(&face.normal.into());
                for &vi in &face.vertices {
                    let p = stl.vertices[vi];
                    verts.push(Vertex {
                        position: vec3(p[0], p[1], p[2]),
                        uv: Vec2::ZERO,
                        color: color.into(),
                        normal: vec4(
                            face.normal[0],
                            face.normal[1],
                            face.normal[2],
                            0.0,
                        ),
                    });
                }
                let b = (i * 3) as u16;
                idxs.extend_from_slice(&[b, b + 1, b + 2]);
            }

            Mesh {
                vertices: verts,
                indices: idxs,
                texture: None,
            }
        })
        .collect()
}

/// Compute the axis-aligned bounding box of all mesh vertices.
fn aabb(stl: &stl_io::IndexedMesh) -> (Vec3, Vec3) {
    stl.vertices.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(lo, hi), v| {
            let p = vec3(v[0], v[1], v[2]);
            (lo.min(p), hi.max(p))
        },
    )
}

// ─── Window config ────────────────────────────────────────────────────────────

fn window_conf() -> Conf {
    Conf {
        window_title: "STL Viewer".to_owned(),
        window_width: 1280,
        window_height: 800,
        ..Default::default()
    }
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[macroquad::main(window_conf)]
async fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "model.stl".to_owned());

    macroquad::window::gl_set_drawcall_buffer_capacity(
        70000,
        70000,
    );

    // Load and parse the STL file
    let mut stl = {
        let f = File::open(&path)
            .unwrap_or_else(|e| panic!("Cannot open {path:?}: {e}"));
        stl_io::read_stl(&mut BufReader::new(f))
            .unwrap_or_else(|e| panic!("Failed to parse {path:?}: {e}"))
    };

    // ── Re-orient: STL is Z-up; our world is Y-up ─────────────────────────
    //
    // We transform the data once here so that every Vec3 you pass to the
    // debug helpers (or inspect in a debugger) is already in Y-up space and
    // the coordinate relationships between vertices/faces are preserved.
    for v in &mut stl.vertices {
        *v = zup_to_yup(*v);
    }
    for face in &mut stl.faces {
        face.normal = zup_to_yup(face.normal);
    }

    println!(
        "Loaded {path:?} — {} faces, {} unique vertices",
        stl.faces.len(),
        stl.vertices.len()
    );

    // Auto-fit the camera around the mesh bounding sphere
    let (lo, hi) = aabb(&stl);
    let center = (lo + hi) * 0.5;
    let radius = (hi - lo).length() * 0.5;
    let diag_extent = (hi - lo).length();

    let batches = build_batches(&stl);
    let mut cam = OrbitCamera::new(center, radius);

    let face_count_str = format!("{} faces", stl.faces.len());

    loop {
        cam.update();

        // ── 3-D pass ──────────────────────────────────────────────────────
        let near = (cam.distance - diag_extent).max(1e-3);
        let far = cam.distance + diag_extent * 2.0;

        set_camera(&Camera3D {
            position: cam.eye(),
            target: cam.target,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 45.0,
            z_near: near,
            z_far: far,
            ..Default::default()
        });

        clear_background(Color::new(0.09, 0.09, 0.12, 1.0));

        // Draw the STL mesh
        for mesh in &batches {
            draw_mesh(mesh);
        }

        // ── Example debug draws (remove or replace with your own) ─────────
        //
        // Occluded by mesh geometry (depth_test = true):
        draw_debug_point(center, radius * 0.02, YELLOW, true);
        //
        // Always on top (depth_test = false):
        draw_debug_edge(lo, hi, Color::new(1.0, 0.3, 0.3, 0.8), true);

        // ── 2-D HUD ───────────────────────────────────────────────────────
        set_default_camera();

        let hud = "Drag: orbit  |  Scroll: zoom  |  R: reset  |  ESC: quit";
        draw_text(hud, 10.0, 24.0, 18.0, Color::new(0.80, 0.80, 0.80, 1.0));
        draw_text(&face_count_str, 10.0, 46.0, 16.0, Color::new(0.55, 0.55, 0.60, 1.0));

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}