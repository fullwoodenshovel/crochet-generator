use anyhow::Result;
use three_d::*;

use crate::camera::OrbitCamera;
use crate::debug::DebugRenderer;
use crate::model::Model;

pub struct Viewer {
    pub model: Model,

    window: Window,
    context: Context,

    cam: OrbitCamera,
    debug: DebugRenderer,
    gui: GUI,

    ambient: AmbientLight,
    directional: DirectionalLight,

    mesh: Gm<Mesh, PhysicalMaterial>,
    index: usize
}

impl Viewer {
    pub fn new(path: &str) -> Result<Self> {
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
            vec3(-1.0, -2.0, -3.0),
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

        let debug = DebugRenderer::new(&context);

        Ok(Self {
            model,
            window,
            context,
            cam,
            debug,
            gui,
            ambient,
            directional,
            mesh,
            index: 0
        })
    }

    pub fn run(mut self) -> Result<()> {
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
                |gui_context| {
                    build_gui(&self.model, &self.cam, gui_context, fps)
                },
            );

            /* ================= DEBUG GEOMETRY ================= */

            if self.index == 0 {
                self.debug.point(
                    self.model.centre,
                    self.model.radius * 0.02,
                    Srgba::new(255, 220, 0, 255),
                    true,
                );
    
                self.debug.edge(
                    self.model.centre,
                    self.model.centre + vec3(self.model.radius, 0.0, 0.0),
                    self.model.radius * 0.01,
                    Srgba::new(255, 80, 80, 200),
                    true,
                );
            }

            if self.index < 300 {
                let l = self.model.mesh.vertices.len();
                for i in 0..100 {
                    let i = i * l / 100 + self.index;
                    let v = self.model.mesh.vertices[i].0;
    
                    self.debug.point(
                        vec3(v[0], v[1], v[2]),
                        self.model.radius * 0.005,
                        Srgba::RED,
                        true,
                    );
                }
    
                self.index += 1;
            }

            self.debug.upload();

            /* ================= RENDER ================= */

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
                .write(|| self.gui.render());

            FrameOutput::default()
        });

        Ok(())
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
            ui.separator();
            ui.label("Drag: orbit   Scroll: zoom");
        });

    let painter = gui_context.debug_painter();

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

    painter.line_segment(
        [
            egui::pos2(50.0, 540.0),
            egui::pos2(440.0, 600.0),
        ],
        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 80, 80)),
    );

    painter.text(
        egui::pos2(50.0, 380.0),
        egui::Align2::LEFT_BOTTOM,
        "raw painter overlay",
        egui::FontId::monospace(14.0),
        egui::Color32::WHITE,
    );
}