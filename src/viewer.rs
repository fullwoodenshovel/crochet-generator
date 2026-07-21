// This file was partially made by AI

use std::sync::Arc;

use anyhow::Result;
use three_d::*;
use tokio::sync::mpsc;
use crate::process::ProcessorCommand;
use crate::camera::OrbitCamera;
use crate::debug::DebugRenderer;
use crate::model::{LIGHT_DIR, Model};

type V3 = stl_io::Vector<f32>;

pub enum DisplayCommand {
    ClearAll,
    Clear(usize),
    Point {
        pos: V3,
        radius: f32,
        colour: Srgba,
        depth: bool,
        group: usize
    },
    Edge {
        a: V3,
        b: V3,
        thickness: f32,
        colour: Srgba,
        depth: bool,
        group: usize
    },
    Face {
        a: V3,
        b: V3,
        c: V3,
        colour: Srgba,
        depth: bool,
        group: usize
    },
}

pub struct Viewer {
    pub model: Arc<Model>,

    window: Window,

    cam: OrbitCamera,
    debug: DebugRenderer,
    gui: GUI,

    ambient: AmbientLight,
    directional: DirectionalLight,

    mesh: Gm<Mesh, PhysicalMaterial>,
    sender: mpsc::UnboundedSender<ProcessorCommand>,
    receiver: mpsc::UnboundedReceiver<DisplayCommand>
}

impl Viewer {
    pub fn new(path: &str, receiver: mpsc::UnboundedReceiver<DisplayCommand>, sender: mpsc::UnboundedSender<ProcessorCommand>) -> Result<Self> {
        let window = Window::new(WindowSettings {
            title: "STL Viewer".to_string(),
            max_size: Some((1920, 1080)),
            ..Default::default()
        })?;

        let context = window.gl();

        let model = Model::load(path)?;

        let viewport = window.viewport();
        let cam = OrbitCamera::new(viewport, model.centre, model.radius);

        let gui = GUI::new(&context);

        let ambient = AmbientLight::new(&context, 0.3, Srgba::WHITE);
        let directional = DirectionalLight::new(
            &context,
            2.0,
            Srgba::WHITE,
            -LIGHT_DIR,
        );

        let cpu_mesh = model.cpu_mesh();
        let gpu_mesh = Mesh::new(&context, &cpu_mesh);

        let mut material = PhysicalMaterial::new_opaque(
            &context,
            &CpuMaterial {
                albedo: Srgba::new(180, 185, 210, 255),
                ..Default::default()
            },
        );

        material.render_states.cull = Cull::None;

        let mesh = Gm::new(gpu_mesh, material);

        Ok(Self {
            model: Arc::new(model),
            window,
            debug: DebugRenderer::new(&context),
            cam,
            gui,
            ambient,
            directional,
            mesh,
            sender,
            receiver,
        })
    }

    pub fn run(mut self) {
        self.window.render_loop(move |mut frame_input| {
            self.cam.resize(frame_input.viewport);
            self.cam.update(&mut frame_input.events);
            self.cam.tighten_clip_planes(self.model.radius);

            let fps = if frame_input.elapsed_time > 0.0 {
                1000.0 / frame_input.elapsed_time
            } else {
                0.0
            };

            self.gui.update(
                &mut frame_input.events,
                frame_input.accumulated_time,
                frame_input.viewport,
                frame_input.device_pixel_ratio,
                |gui_context| build_gui(&self.model, &self.cam, gui_context, fps),
            );

            /* ================= DEBUG ================= */

            while let Ok(command) = self.receiver.try_recv() {
                match command {
                    DisplayCommand::ClearAll => self.debug.clear_all(),
                    DisplayCommand::Clear(name) => self.debug.clear(name),
                    DisplayCommand::Point { pos, radius, colour, depth, group } => self.debug.point(group, pos.0.into(), radius, colour, depth),
                    DisplayCommand::Edge { a, b, thickness, colour, depth, group } => self.debug.edge(group, a.0.into(), b.0.into(), thickness, colour, depth),
                    DisplayCommand::Face { a, b, c, colour, depth, group } => self.debug.face(group, a.0.into(), b.0.into(), c.0.into(), colour, depth),
                }
            }

            /* IMPORTANT: correct pipeline */
            self.debug.upload();

            for event in &frame_input.events {
                if let Event::MousePress {
                    button,
                    position,
                    handled: false,
                    ..
                } = *event
                {
                    let dir = self.cam.camera.view_direction_at_pixel(position);
                    let origin = self.cam.camera.position_at_pixel(position);

                    if let Some((face, hit)) = pick_triangle(&self.model, origin, dir) {
                        self.sender.send(ProcessorCommand::MouseDownOnPoint { face_index: face, position: V3::new(hit.into()), button }).unwrap_or_default();
                    }
                }
            }

            let _ = frame_input
                .screen()
                .clear(ClearState::color_and_depth(
                    0.09, 0.09, 0.12, 1.0, 1.0,
                ))
                .render(
                    &self.cam.camera,
                    std::iter::once(&self.mesh as &dyn Object)
                        .chain(self.debug.occluded()),
                    &[&self.ambient, &self.directional],
                )
                .clear(ClearState::depth(1.0))
                .render(
                    &self.cam.camera,
                    self.debug.overlay(),
                    &[&self.ambient, &self.directional],
                )
                .write(|| self.gui.render());

            FrameOutput::default()
            
        });
    }
}

/* ================= GUI ================= */

fn build_gui(
    model: &Model,
    cam: &OrbitCamera,
    gui_context: &egui::Context,
    fps: f64,
) {
    egui::Window::new("Debug")
        .default_pos(egui::pos2(10.0, 60.0))
        .resizable(false)
        .show(gui_context, |ui| {
            ui.label(format!("FPS: {fps:.1}"));
            ui.label(format!("Faces: {}", model.mesh.faces.len()));
            ui.label(format!("Vertices: {}", model.mesh.vertices.len()));
            ui.label(format!(
                "Camera distance: {:.3}",
                cam.camera.position().distance(model.centre)
            ));
        });

    let painter = gui_context.debug_painter();

    // RECTANGLE (same as before)
    painter.rect_stroke(
        egui::Rect::from_min_size(
            egui::pos2(50.0, 400.0),
            egui::vec2(220.0, 100.0),
        ),
        4.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 160, 40)),
        egui::StrokeKind::Middle,
    );

    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(320.0, 400.0),
            egui::vec2(120.0, 60.0),
        ),
        4.0,
        egui::Color32::from_rgba_unmultiplied(80, 180, 255, 160),
    );

    // LINE
    painter.line_segment(
        [egui::pos2(50.0, 540.0), egui::pos2(440.0, 600.0)],
        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 80, 80)),
    );

    // TEXT
    painter.text(
        egui::pos2(50.0, 380.0),
        egui::Align2::LEFT_BOTTOM,
        "raw painter overlay",
        egui::FontId::monospace(14.0),
        egui::Color32::WHITE,
    );
}

fn pick_triangle(
    model: &Model,
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<(usize, Vec3)> {
    let mut closest_t = f32::INFINITY;
    let mut closest = None;

    for (face_index, face) in model.mesh.faces.iter().enumerate() {
        let a: Vec3 = model.mesh.vertices[face.vertices[0]].0.into();
        let b: Vec3 = model.mesh.vertices[face.vertices[1]].0.into();
        let c: Vec3 = model.mesh.vertices[face.vertices[2]].0.into();

        if let Some(t) = ray_triangle(ray_origin, ray_dir, a, b, c) {
            if t < closest_t {
                closest_t = t;
                closest = Some((face_index, ray_origin + ray_dir * t));
            }
        }
    }

    closest
}

fn ray_triangle(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<f32> {
    const EPS: f32 = 1e-6;

    let edge1 = b - a;
    let edge2 = c - a;

    let h = direction.cross(edge2);
    let det = edge1.dot(h);

    if det.abs() < EPS {
        return None;
    }

    let inv_det = 1.0 / det;

    let s = origin - a;
    let u = inv_det * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = inv_det * direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = inv_det * edge2.dot(q);

    if t > EPS {
        Some(t)
    } else {
        None
    }
}