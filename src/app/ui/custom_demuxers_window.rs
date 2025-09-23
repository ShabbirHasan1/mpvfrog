use {
    crate::{
        app::Core,
        config::{Command, CustomPlayerEntry, HasExtsPredicate, Predicate, PredicateKind},
    },
    egui_sf2g::egui::{self, Color32, ComboBox, Context, RichText, ScrollArea, Ui, Window},
};

#[derive(Default)]
pub struct CustomDemuxersWindow {
    pub open: bool,
    edit_buffer: String,
    edit_target: Option<EditTarget>,
    error_label: String,
    selected_idx: usize,
    tab: CustomDemuxTab,
    selected_pred_idx: usize,
}

#[derive(PartialEq, Default)]
enum CustomDemuxTab {
    #[default]
    Commands,
    Predicates,
}

struct EditTarget {
    index: usize,
    which: EditTargetWhich,
}

enum EditTargetWhich {
    Command,
    MpvArgs,
}

impl PredicateKind {
    fn label(&self) -> &str {
        match self {
            Self::BeginsWith => "Begins with",
            Self::HasExts => "Has extension(s)",
        }
    }
    fn desc(&self) -> &'static str {
        match self {
            Self::BeginsWith => "Begins with a string (e.g. `mdat.`) for TFMX files",
            Self::HasExts => {
                "Space separated list of file extensions (e.g. `mod xm it`) for module files"
            }
        }
    }
}

impl CustomDemuxersWindow {
    pub(super) fn update(&mut self, core: &mut Core, ctx: &Context) {
        let mut open = self.open;
        Window::new("Custom demuxers")
            .open(&mut open)
            .show(ctx, |ui| self.window_ui(core, ui));
        self.open = open;
    }
    fn window_ui(&mut self, core: &mut Core, ui: &mut Ui) {
        let mut idx = 0;
        enum Op {
            None,
            Swap(usize, usize),
            Clone(usize),
        }
        let mut op = Op::None;
        ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            let len = core.cfg.custom_demuxers.len();
            core.cfg.custom_demuxers.retain_mut(|custom_player| {
                let mut retain = true;
                ui.horizontal(|ui| {
                    let label = if custom_player.name.is_empty() {
                        "<unnamed demuxer>"
                    } else {
                        &custom_player.name
                    };
                    if ui
                        .selectable_label(self.selected_idx == idx, label)
                        .clicked()
                    {
                        self.selected_idx = idx;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").on_hover_text("Delete").clicked() {
                            retain = false;
                        }
                        if ui.button("⏶").on_hover_text("Higher prio").clicked() && idx > 0 {
                            op = Op::Swap(idx, idx - 1);
                        }
                        if ui.button("⏷").on_hover_text("Lower prio").clicked() && idx < len - 1 {
                            op = Op::Swap(idx, idx + 1);
                        }
                        if ui.button("🗐").on_hover_text("Clone").clicked() {
                            op = Op::Clone(idx);
                        }
                    });
                    idx += 1;
                });
                retain
            });
        });
        match op {
            Op::None => {}
            Op::Swap(a, b) => core.cfg.custom_demuxers.swap(a, b),
            Op::Clone(idx) => core
                .cfg
                .custom_demuxers
                .insert(idx, core.cfg.custom_demuxers[idx].clone()),
        }
        if ui.button("➕ Add").clicked() {
            core.cfg.custom_demuxers.push(CustomPlayerEntry::default());
        }
        ui.separator();
        if let Some(custom) = core.cfg.custom_demuxers.get_mut(self.selected_idx) {
            self.selected_demuxer_ui(ui, idx, custom);
        }
    }

    fn selected_demuxer_ui(&mut self, ui: &mut Ui, idx: usize, custom: &mut CustomPlayerEntry) {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.add(egui::TextEdit::singleline(&mut custom.name).desired_width(f32::INFINITY));
        });
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.tab == CustomDemuxTab::Commands, "Commands")
                .clicked()
            {
                self.tab = CustomDemuxTab::Commands;
            }
            if ui
                .selectable_label(self.tab == CustomDemuxTab::Predicates, "Predicates")
                .clicked()
            {
                self.tab = CustomDemuxTab::Predicates;
            }
        });
        ui.separator();
        match self.tab {
            CustomDemuxTab::Commands => {
                ui.label("Demuxer command");
                match self.edit_target {
                    Some(EditTarget {
                        index,
                        which: EditTargetWhich::Command,
                    }) if idx == index => {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut self.edit_buffer)
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            )
                            .lost_focus()
                        {
                            match Command::from_str(&self.edit_buffer) {
                                Ok(cmd) => {
                                    custom.reader_cmd = cmd;
                                    self.error_label.clear();
                                }
                                Err(e) => self.error_label = e.to_string(),
                            }
                            self.edit_buffer.clear();
                            self.edit_target = None;
                        }
                    }
                    _ => {
                        if ui
                            .add(
                                egui::TextEdit::multiline(
                                    &mut custom.reader_cmd.to_string().unwrap(),
                                )
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                            )
                            .gained_focus()
                        {
                            self.edit_buffer = custom.reader_cmd.to_string().unwrap();
                            self.edit_target = Some(EditTarget {
                                index: idx,
                                which: EditTargetWhich::Command,
                            });
                        }
                    }
                };
                if !self.error_label.is_empty() {
                    ui.label(RichText::new(&self.error_label).color(Color32::RED));
                }
                ui.label("Example: my-cmd --input {}");
                ui.label("extra mpv args");
                match self.edit_target {
                    Some(EditTarget {
                        index,
                        which: EditTargetWhich::MpvArgs,
                    }) if idx == index => {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut self.edit_buffer)
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            )
                            .lost_focus()
                        {
                            custom.extra_mpv_args = self
                                .edit_buffer
                                .split_whitespace()
                                .map(String::from)
                                .collect();
                            self.edit_buffer.clear();
                            self.edit_target = None;
                        }
                    }
                    _ => {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut custom.extra_mpv_args.join(" "))
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            )
                            .gained_focus()
                        {
                            self.edit_buffer = custom.extra_mpv_args.join(" ");
                            self.edit_target = Some(EditTarget {
                                index: idx,
                                which: EditTargetWhich::MpvArgs,
                            });
                        }
                    }
                }
            }
            CustomDemuxTab::Predicates => {
                ui.horizontal(|ui| {
                    for i in 0..custom.predicates.len() {
                        if ui
                            .selectable_label(i == self.selected_pred_idx, (i + 1).to_string())
                            .clicked()
                        {
                            self.selected_pred_idx = i;
                        }
                    }
                    if ui.button("➕").on_hover_text("Add predicate").clicked() {
                        custom
                            .predicates
                            .push(Predicate::HasExts(HasExtsPredicate::default()));
                    }
                });
                if let Some(pred) = custom.predicates.get_mut(self.selected_pred_idx) {
                    let re = ComboBox::new(idx, "Kind")
                        .selected_text(PredicateKind::from(&*pred).label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                pred,
                                Predicate::BeginsWith(String::new()),
                                PredicateKind::BeginsWith.label(),
                            );
                            ui.selectable_value(
                                pred,
                                Predicate::HasExts(HasExtsPredicate::default()),
                                PredicateKind::HasExts.label(),
                            );
                        });
                    let desc = PredicateKind::from(&*pred).desc();
                    re.response.on_hover_text(desc);
                    match pred {
                        Predicate::BeginsWith(frag) => {
                            ui.add(
                                egui::TextEdit::singleline(frag)
                                    .hint_text(desc)
                                    .desired_width(f32::INFINITY),
                            );
                        }
                        Predicate::HasExts(HasExtsPredicate {
                            ext_list,
                            case_sensitive,
                        }) => {
                            ui.add(
                                egui::TextEdit::singleline(ext_list)
                                    .hint_text(desc)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.checkbox(case_sensitive, "Case sensitive");
                        }
                    };
                    if ui.button("Remove").clicked() {
                        custom.predicates.remove(self.selected_pred_idx);
                    }
                }
            }
        }
    }
}
