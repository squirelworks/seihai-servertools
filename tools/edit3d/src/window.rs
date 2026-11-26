//! The editor window. Every rule is in `lat_core::world3d`; this only
//! shows the world, turns clicks into edits and redraws the preview.

use eframe::egui;
use egui::{Color32, PointerButton, Sense, TextureOptions, Vec2};

use lat_core::world3d::doc::{Object, Turn, WorldDoc};
use lat_core::world3d::terrain::Field;
use lat_core::world3d::view::{self, Camera};
use lat_core::world3d::{world_names, Brush, World, SPAWN_MARKER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Select,
    Place,
    Terrain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrushKind {
    Raise,
    Lower,
    Flatten,
    Smooth,
    Paint,
}

/// What an undo puts back.
#[derive(Clone)]
struct Snapshot {
    doc: WorldDoc,
    terrain: Option<Field>,
}

struct Editor {
    world: World,
    renderer: Option<String>,
    /// The orbit camera: the point looked at, distance, yaw, pitch.
    target: [f64; 3],
    distance: f64,
    yaw: f64,
    pitch: f64,
    texture: Option<egui::TextureHandle>,
    /// Per pixel, the object drawn there (from the last frame).
    picks: Vec<u32>,
    frame_size: [u32; 2],
    redraw: bool,
    selected: Option<String>,
    mode: Mode,
    place_shape: String,
    brush: BrushKind,
    radius: f64,
    amount: f64,
    paint: u8,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    doc_dirty: bool,
    terrain_dirty: bool,
    status: String,
    notes: Vec<String>,
    filter: String,
    /// Asked to switch world while there are unsaved changes.
    pending_world: Option<String>,
}

pub fn run(world: World, renderer: Option<String>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Seihai 3D level editor")
            .with_inner_size([1200.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Seihai 3D level editor",
        options,
        Box::new(move |_cc| Ok(Box::new(Editor::new(world, renderer)))),
    )
}

impl Editor {
    fn new(world: World, renderer: Option<String>) -> Self {
        let target = world.doc.spawn().unwrap_or([0.0, 0.0, 0.0]);
        let place_shape = world.shape_files().into_iter().next().unwrap_or_default();
        Editor {
            world,
            renderer,
            target,
            distance: 40.0,
            yaw: 0.0,
            pitch: 0.6,
            texture: None,
            picks: Vec::new(),
            frame_size: [0, 0],
            redraw: true,
            selected: None,
            mode: Mode::Select,
            place_shape,
            brush: BrushKind::Raise,
            radius: 16.0,
            amount: 2.0,
            paint: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            doc_dirty: false,
            terrain_dirty: false,
            status: String::new(),
            notes: Vec::new(),
            filter: String::new(),
            pending_world: None,
        }
    }

    fn camera(&self) -> Camera {
        Camera::orbit(self.target, self.distance, self.yaw, self.pitch)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: self.world.doc.clone(),
            terrain: self.world.terrain.clone(),
        }
    }

    /// Call before every change.
    fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 40 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn restore(&mut self, s: Snapshot) {
        if s.terrain != self.world.terrain {
            self.terrain_dirty = true;
        }
        let terrain_line_changed = s.doc.terrain() != self.world.doc.terrain();
        self.world.doc = s.doc;
        self.world.terrain = s.terrain;
        if terrain_line_changed {
            self.world.reload_terrain();
        }
        self.doc_dirty = true;
        self.redraw = true;
    }

    fn dirty(&self) -> bool {
        self.doc_dirty || self.terrain_dirty
    }

    fn draw(&mut self, ctx: &egui::Context, size: [u32; 2]) {
        let cam = self.camera();
        let image = match &self.renderer {
            Some(cmd) => {
                match crate::external_render(cmd, &self.world, &cam, size[0], size[1]).and_then(
                    |png| lat_core::render::decode_image(&png).ok_or("not a PNG".to_string()),
                ) {
                    Ok(img) => {
                        self.picks.clear();
                        img
                    }
                    Err(e) => {
                        self.status = format!("renderer: {e}; using the built-in preview");
                        self.renderer = None;
                        return self.draw(ctx, size);
                    }
                }
            }
            None => {
                let (scene, notes) = self.world.scene(2);
                self.notes = notes;
                let frame = view::render(&scene, &cam, size[0], size[1]);
                self.picks = frame.objects;
                frame.image
            }
        };
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [image.width as usize, image.height as usize],
            &image.pixels,
        );
        match &mut self.texture {
            Some(t) => t.set(color, TextureOptions::LINEAR),
            None => self.texture = Some(ctx.load_texture("preview", color, TextureOptions::LINEAR)),
        }
        self.frame_size = [image.width, image.height];
        self.redraw = false;
    }

    fn save(&mut self) {
        match crate::save(&self.world, self.terrain_dirty) {
            Ok(said) => {
                self.doc_dirty = false;
                self.terrain_dirty = false;
                self.status = said.join("; ");
            }
            Err(e) => self.status = format!("not saved: {e}"),
        }
    }

    fn switch_world(&mut self, name: &str) {
        match World::open(&self.world.root, name) {
            Ok(w) => {
                self.target = w.doc.spawn().unwrap_or([0.0, 0.0, 0.0]);
                self.world = w;
                self.selected = None;
                self.undo.clear();
                self.redo.clear();
                self.doc_dirty = false;
                self.terrain_dirty = false;
                self.redraw = true;
                self.status = format!("opened {name}");
            }
            Err(e) => self.status = e,
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let names = world_names(&self.world.root);
            let mut pick = self.world.name.clone();
            egui::ComboBox::from_id_salt("world")
                .selected_text(&pick)
                .show_ui(ui, |ui| {
                    for n in &names {
                        ui.selectable_value(&mut pick, n.clone(), n);
                    }
                });
            if pick != self.world.name {
                if self.dirty() {
                    self.pending_world = Some(pick);
                } else {
                    self.switch_world(&pick);
                }
            }
            let save_text = if self.dirty() { "Save *" } else { "Save" };
            if ui.button(save_text).clicked() {
                self.save();
            }
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                let s = self.undo.pop().expect("an undo");
                self.redo.push(self.snapshot());
                self.restore(s);
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                .clicked()
            {
                let s = self.redo.pop().expect("a redo");
                self.undo.push(self.snapshot());
                self.restore(s);
            }
            if ui.button("Reload files").clicked() {
                self.world.rescan();
                self.redraw = true;
            }
            ui.separator();
            ui.selectable_value(&mut self.mode, Mode::Select, "Select");
            ui.selectable_value(&mut self.mode, Mode::Place, "Place");
            ui.add_enabled_ui(self.world.terrain.is_some(), |ui| {
                ui.selectable_value(&mut self.mode, Mode::Terrain, "Terrain");
            });
            ui.separator();
            ui.label(self.world.file.display().to_string());
        });
        if let Some(name) = self.pending_world.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(
                    Color32::from_rgb(220, 120, 0),
                    format!("Unsaved changes. Open {name} anyway?"),
                );
                if ui.button("Save first").clicked() {
                    self.save();
                    if !self.dirty() {
                        self.switch_world(&name);
                    }
                    self.pending_world = None;
                }
                if ui.button("Discard").clicked() {
                    self.switch_world(&name);
                    self.pending_world = None;
                }
                if ui.button("Stay").clicked() {
                    self.pending_world = None;
                }
            });
        }
    }

    fn object_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Objects");
        ui.text_edit_singleline(&mut self.filter);
        let objects = self.world.doc.objects();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for o in &objects {
                if !self.filter.is_empty()
                    && !o.name.to_lowercase().contains(&self.filter.to_lowercase())
                {
                    continue;
                }
                let sel = self.selected.as_deref() == Some(o.name.as_str());
                if ui
                    .selectable_label(sel, format!("{}  ({})", o.name, o.shape))
                    .clicked()
                {
                    self.selected = Some(o.name.clone());
                    self.target = o.pos;
                    self.redraw = true;
                }
            }
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        match self.mode {
            Mode::Place => {
                ui.heading("Place");
                ui.label("Click the ground to add this shape there.");
                let shapes = self.world.shape_files();
                egui::ComboBox::from_id_salt("shape")
                    .selected_text(&self.place_shape)
                    .show_ui(ui, |ui| {
                        for s in shapes {
                            ui.selectable_value(&mut self.place_shape, s.clone(), s);
                        }
                    });
                ui.text_edit_singleline(&mut self.place_shape);
                ui.separator();
            }
            Mode::Terrain => {
                ui.heading("Terrain");
                for (b, label) in [
                    (BrushKind::Raise, "Raise"),
                    (BrushKind::Lower, "Lower"),
                    (BrushKind::Flatten, "Flatten to"),
                    (BrushKind::Smooth, "Smooth"),
                    (BrushKind::Paint, "Paint"),
                ] {
                    ui.radio_value(&mut self.brush, b, label);
                }
                ui.add(egui::Slider::new(&mut self.radius, 1.0..=256.0).text("radius"));
                match self.brush {
                    BrushKind::Paint => {
                        let names = self
                            .world
                            .terrain
                            .as_ref()
                            .map(|t| t.names.clone())
                            .unwrap_or_default();
                        for (i, n) in names.iter().enumerate() {
                            ui.radio_value(&mut self.paint, i as u8, n);
                        }
                    }
                    BrushKind::Smooth => {
                        self.amount = self.amount.clamp(0.0, 1.0);
                        ui.add(egui::Slider::new(&mut self.amount, 0.0..=1.0).text("strength"));
                    }
                    _ => {
                        ui.add(egui::Slider::new(&mut self.amount, 0.0..=200.0).text("units"));
                    }
                }
                if let Some(t) = &self.world.terrain {
                    if t.missing > 0 {
                        ui.label(format!("{} chunks were missing (flat at 0)", t.missing));
                    }
                }
                ui.separator();
            }
            Mode::Select => {}
        }

        if let Some(name) = self.selected.clone() {
            if let Some(mut o) = self.world.doc.object(&name) {
                ui.heading(&o.name);
                let before = o.clone();
                let mut new_name = o.name.clone();
                ui.horizontal(|ui| {
                    ui.label("name");
                    ui.text_edit_singleline(&mut new_name);
                });
                ui.horizontal(|ui| {
                    ui.label("shape");
                    ui.text_edit_singleline(&mut o.shape);
                });
                ui.horizontal(|ui| {
                    for (i, l) in ["x", "y", "z"].iter().enumerate() {
                        ui.add(egui::DragValue::new(&mut o.pos[i]).speed(0.1).prefix(*l));
                    }
                });
                let mut deg = o
                    .turn
                    .filter(|t| t.axis == [0.0, 0.0, 1.0])
                    .map_or(0.0, |t| t.angle.to_degrees());
                let other_axis = o.turn.is_some_and(|t| t.axis != [0.0, 0.0, 1.0]);
                if other_axis {
                    ui.label("turned about another axis (edit the file to change it)");
                } else if ui
                    .add(egui::DragValue::new(&mut deg).speed(1.0).suffix("° turn"))
                    .changed()
                {
                    o.turn = if deg == 0.0 {
                        None
                    } else {
                        Some(Turn::about_z(deg.to_radians()))
                    };
                }
                if ui.button("Stand on the ground").clicked() {
                    o.pos[2] = self.world.ground(o.pos[0], o.pos[1]);
                }
                if new_name != o.name && !new_name.is_empty() && !new_name.contains(' ') {
                    o.name = new_name;
                }
                if o != before {
                    self.checkpoint();
                    match self.world.doc.set_object(&name, &o) {
                        Ok(()) => {
                            self.selected = Some(o.name.clone());
                            self.doc_dirty = true;
                            self.redraw = true;
                        }
                        Err(e) => {
                            self.undo.pop();
                            self.status = e;
                        }
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() {
                        let mut c = before.clone();
                        c.name = self.world.doc.free_name(&before.name);
                        c.pos[0] += 2.0;
                        self.checkpoint();
                        if self.world.doc.add_object(&c).is_ok() {
                            self.selected = Some(c.name);
                            self.doc_dirty = true;
                            self.redraw = true;
                        }
                    }
                    if ui.button("Delete").clicked() {
                        self.checkpoint();
                        let _ = self.world.doc.remove_object(&before.name);
                        self.selected = None;
                        self.doc_dirty = true;
                        self.redraw = true;
                    }
                });
                ui.separator();
            } else {
                self.selected = None;
            }
        }

        ui.heading("World");
        let mut spawn = self.world.doc.spawn().unwrap_or([0.0, 0.0, 0.0]);
        let old = spawn;
        ui.horizontal(|ui| {
            ui.label("spawn");
            for v in spawn.iter_mut() {
                ui.add(egui::DragValue::new(v).speed(0.1));
            }
        });
        if ui.button("Spawn at the view's centre").clicked() {
            spawn = [
                self.target[0],
                self.target[1],
                self.world.ground(self.target[0], self.target[1]) + 1.0,
            ];
        }
        if spawn != old {
            self.checkpoint();
            self.world.doc.set_spawn(spawn);
            self.doc_dirty = true;
            self.redraw = true;
        }
        let mut sun = self.world.doc.sun().unwrap_or_default();
        let old_sun = sun;
        ui.horizontal(|ui| {
            ui.label("sun toward");
            for v in sun.dir.iter_mut() {
                ui.add(egui::DragValue::new(v).speed(0.02).range(-1.0..=1.0));
            }
        });
        ui.horizontal(|ui| {
            ui.label("light");
            for v in sun.colour.iter_mut().take(3) {
                ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0));
            }
        });
        ui.horizontal(|ui| {
            ui.label("ambient");
            for v in sun.ambient.iter_mut().take(3) {
                ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0));
            }
        });
        if sun != old_sun {
            self.checkpoint();
            self.world.doc.set_sun(&sun);
            self.doc_dirty = true;
            self.redraw = true;
        }

        let problems = self.world.doc.problems();
        if !problems.is_empty() || !self.notes.is_empty() {
            ui.separator();
            ui.heading("Problems");
            egui::ScrollArea::vertical()
                .id_salt("problems")
                .max_height(160.0)
                .show(ui, |ui| {
                    for p in problems.iter().chain(&self.notes) {
                        ui.label(p);
                    }
                });
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let avail = ui.available_size();
        // Drawn at most 960 wide; egui scales it to the panel.
        let scale = (960.0 / avail.x).min(1.0);
        let size = [
            ((avail.x * scale) as u32).max(16),
            ((avail.y * scale) as u32).max(16),
        ];
        if self.redraw || size != self.frame_size {
            self.draw(ctx, size);
        }
        let (rect, resp) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        if let Some(t) = &self.texture {
            ui.painter().image(
                t.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let shift = ui.input(|i| i.modifiers.shift);
        let delta: Vec2 = resp.drag_delta();
        if resp.dragged_by(PointerButton::Middle)
            || (shift && resp.dragged_by(PointerButton::Secondary))
        {
            let k = self.distance / 400.0;
            let (s, c) = self.yaw.sin_cos();
            let (dx, dy) = (f64::from(-delta.x) * k, f64::from(delta.y) * k);
            self.target[0] += dx * c + dy * s;
            self.target[1] += -dx * s + dy * c;
            self.redraw = true;
        } else if resp.dragged_by(PointerButton::Secondary) {
            self.yaw += f64::from(delta.x) * 0.01;
            self.pitch = (self.pitch + f64::from(delta.y) * 0.01).clamp(-1.5, 1.55);
            self.redraw = true;
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.distance =
                    (self.distance * (1.0 - f64::from(scroll) * 0.002)).clamp(2.0, 4000.0);
                self.redraw = true;
            }
        }
        // Where the pointer is, in frame pixels.
        let at = resp.interact_pointer_pos().map(|p| {
            let fx = f64::from((p.x - rect.min.x) / rect.width()) * f64::from(self.frame_size[0]);
            let fy = f64::from((p.y - rect.min.y) / rect.height()) * f64::from(self.frame_size[1]);
            (fx, fy)
        });
        let Some((fx, fy)) = at else { return };
        let cam = self.camera();
        let ray = cam.ray(fx, fy, self.frame_size[0], self.frame_size[1]);
        match self.mode {
            Mode::Select if resp.clicked_by(PointerButton::Primary) => {
                let i = fy as usize * self.frame_size[0] as usize + fx as usize;
                let hit = self.picks.get(i).copied().unwrap_or(0);
                let objects = self.world.doc.objects();
                self.selected = if hit == 0 || hit as usize - 1 == SPAWN_MARKER {
                    None
                } else {
                    objects.get(hit as usize - 1).map(|o| o.name.clone())
                };
            }
            Mode::Place if resp.clicked_by(PointerButton::Primary) => {
                if self.place_shape.is_empty() {
                    self.status = "pick a shape to place first".into();
                } else if let Some(p) = self.world.hit_ground(cam.eye, ray) {
                    let base = self
                        .place_shape
                        .rsplit(['/', '\\'])
                        .next()
                        .unwrap_or("obj")
                        .split('.')
                        .next()
                        .unwrap_or("obj")
                        .to_string();
                    let o = Object {
                        only_for: None,
                        name: self.world.doc.free_name(&base),
                        shape: self.place_shape.clone(),
                        pos: p,
                        turn: None,
                        rest: Vec::new(),
                    };
                    self.checkpoint();
                    match self.world.doc.add_object(&o) {
                        Ok(()) => {
                            self.selected = Some(o.name);
                            self.doc_dirty = true;
                            self.redraw = true;
                        }
                        Err(e) => {
                            self.undo.pop();
                            self.status = e;
                        }
                    }
                }
            }
            Mode::Terrain
                if resp.clicked_by(PointerButton::Primary)
                    || resp.dragged_by(PointerButton::Primary) =>
            {
                let Some(p) = self.world.hit_ground(cam.eye, ray) else {
                    return;
                };
                let brush = match self.brush {
                    BrushKind::Raise => Brush::Raise(self.amount * 0.1),
                    BrushKind::Lower => Brush::Raise(-self.amount * 0.1),
                    BrushKind::Flatten => Brush::Flatten(self.amount),
                    BrushKind::Smooth => Brush::Smooth(self.amount),
                    BrushKind::Paint => Brush::Paint(self.paint),
                };
                // One undo step a stroke.
                if resp.drag_started_by(PointerButton::Primary) || resp.clicked() {
                    self.checkpoint();
                }
                if let Some(t) = self.world.terrain.as_mut() {
                    if t.apply(brush, p[0], p[1], self.radius) > 0 {
                        self.terrain_dirty = true;
                        self.redraw = true;
                    }
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for Editor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("bar").show(ctx, |ui| self.top_bar(ui));
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(if self.status.is_empty() {
                "Right-drag turns, middle-drag moves, wheel zooms."
            } else {
                &self.status
            });
        });
        egui::SidePanel::left("objects")
            .default_width(220.0)
            .show(ctx, |ui| self.object_list(ui));
        egui::SidePanel::right("props")
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| self.preview(ui, ctx));
    }
}
