use crate::{App, app_tests, i18n};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, vec2};

fn frame(app: &mut App, ctx: &egui::Context, mut input: egui::RawInput) -> egui::FullOutput {
    input
        .screen_rect
        .get_or_insert(Rect::from_min_size(Pos2::ZERO, vec2(1080., 720.)));
    let mut frame = eframe::Frame::_new_kittest();
    eframe::App::raw_input_hook(app, ctx, &mut input);
    ctx.run(input, |ctx| eframe::App::update(app, ctx, &mut frame))
}

#[test]
fn tab_keyboard_cycle_preserves_dirty_format_history_and_wraps() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    let original = app.docs[0].content.clone();
    app.docs[0].history.record(original);
    app.docs[0].content = crate::core::Content {
        text: "中文🙂\n".into(),
        bom: true,
        newline: crate::core::Newline::Crlf,
    };
    let before = app.docs[0].content.clone();
    app.new_doc();
    app.activate(0);
    frame(&mut app, &ctx, Default::default());
    for (modifiers, active) in [
        (Modifiers::CTRL, 1),
        (Modifiers::CTRL, 0),
        (Modifiers::CTRL | Modifiers::SHIFT, 1),
        (Modifiers::CTRL | Modifiers::SHIFT, 0),
    ] {
        frame(
            &mut app,
            &ctx,
            egui::RawInput {
                modifiers,
                events: vec![Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
        );
        assert_eq!(app.active, active, "tab cycling did not consume shortcut");
        assert_eq!(app.docs[0].content, before);
        assert!(app.docs[0].dirty());
    }
    let d = &mut app.docs[0];
    d.history.undo(&mut d.content);
    assert_eq!(d.content, d.saved);
}

#[test]
fn tab_middle_close_targets_inactive_dirty_document_and_cancel_keeps_it() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].content.text = "keep 中文🙂".into();
    let dirty_id = app.docs[0].id;
    app.new_doc();
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    let pos = text_center(&output, "● Untitled 1");
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            egui::RawInput {
                events: vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Middle,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
        );
    }
    assert_eq!(app.pending_close, Some(dirty_id));
    assert_eq!(app.active, 1);
    assert_eq!(app.docs.len(), 2);
    let output = frame(&mut app, &ctx, Default::default());
    let cancel = text_center(&output, "Cancel");
    click(&mut app, &ctx, cancel);
    assert_eq!(app.pending_close, None);
    assert_eq!(app.docs[0].content.text, "keep 中文🙂");
}
fn text_center(output: &egui::FullOutput, text: &str) -> Pos2 {
    fn find(shape: &egui::epaint::Shape, text: &str) -> Option<Pos2> {
        match shape {
            egui::epaint::Shape::Text(shape) if shape.galley.text() == text => Some(
                shape.pos
                    + shape.galley.rows[0]
                        .rect_without_leading_space()
                        .center()
                        .to_vec2(),
            ),
            egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, text)),
            _ => None,
        }
    }
    for clipped in output.shapes.iter().rev() {
        if let Some(pos) = find(&clipped.shape, text) {
            return pos;
        }
    }
    panic!("GUI text not found: {text}");
}
fn pointer(
    app: &mut App,
    ctx: &egui::Context,
    pos: Pos2,
    pressed: Option<bool>,
) -> egui::FullOutput {
    let mut events = vec![Event::PointerMoved(pos)];
    if let Some(pressed) = pressed {
        events.push(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    frame(
        app,
        ctx,
        egui::RawInput {
            events,
            ..Default::default()
        },
    )
}
fn click(app: &mut App, ctx: &egui::Context, pos: Pos2) -> egui::FullOutput {
    pointer(app, ctx, pos, Some(true));
    pointer(app, ctx, pos, Some(false))
}

#[test]
fn tab_drag_pointer_release_reorders_and_escape_cancels() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.new_doc();
    let active = app.docs[app.active].id;
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    let from = text_center(&output, "Untitled 1");
    let to = text_center(&output, "Untitled 2") + vec2(3., 0.);
    pointer(&mut app, &ctx, from, Some(true));
    pointer(&mut app, &ctx, to, None);
    pointer(&mut app, &ctx, to, None);
    pointer(&mut app, &ctx, to, Some(false));
    assert_eq!(
        app.docs.iter().map(|doc| doc.id).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(app.docs[app.active].id, active);
    let output = frame(&mut app, &ctx, Default::default());
    let from = text_center(&output, "Untitled 1");
    let to = text_center(&output, "Untitled 2") - vec2(3., 0.);
    pointer(&mut app, &ctx, from, Some(true));
    pointer(&mut app, &ctx, to, None);
    frame(
        &mut app,
        &ctx,
        egui::RawInput {
            events: vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
    );
    pointer(&mut app, &ctx, to, Some(false));
    assert_eq!(
        app.docs.iter().map(|doc| doc.id).collect::<Vec<_>>(),
        [2, 1]
    );
}

#[test]
fn window_menu_switch_and_status_double_click_keep_document_data() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].content.text = "中文🙂\n".into();
    let before = app.docs[0].content.clone();
    app.new_doc();
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    let menu = text_center(&output, "Window");
    click(&mut app, &ctx, menu);
    let output = frame(&mut app, &ctx, Default::default());
    let tab = text_center(&output, "● Untitled 1");
    click(&mut app, &ctx, tab);
    assert_eq!(app.active, 0);
    assert_eq!(app.docs[0].content, before);
    assert!(app.docs[0].dirty());
    frame(&mut app, &ctx, Default::default());
    let output = frame(
        &mut app,
        &ctx,
        egui::RawInput {
            time: Some(ctx.input(|input| input.time) + 1.),
            ..Default::default()
        },
    );
    let position = text_center(&output, "Ln 1 : Col 1");
    click(&mut app, &ctx, position);
    click(&mut app, &ctx, position);
    assert!(app.goto_open);
    assert_eq!(app.docs[0].content, before);
}

#[test]
fn ui_language_menu_saves_preference_and_updates_title_and_controls() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("InkPage.settings");
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.preferences = i18n::Preferences::load(Some(path.clone())).0;
    let output = frame(&mut app, &ctx, Default::default());
    let menu = text_center(&output, "UI language");
    click(&mut app, &ctx, menu);
    let output = frame(&mut app, &ctx, Default::default());
    let japanese = text_center(&output, "日本語");
    click(&mut app, &ctx, japanese);
    let output = frame(&mut app, &ctx, Default::default());
    assert_eq!(app.locale, i18n::Locale::Japanese);
    text_center(&output, "表示言語");
    assert!(output.viewport_output[&egui::ViewportId::ROOT].commands.iter()
        .any(|command| matches!(command, egui::ViewportCommand::Title(title) if title == "InkPage — テキストエディター")));
    assert_eq!(
        i18n::Preferences::load(Some(path)).1,
        i18n::Locale::Japanese
    );
}

#[test]
fn raw_drop_event_reaches_background_open_and_keeps_unsaved_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("拖入🙂.txt");
    std::fs::write(&path, "\u{feff}中文🙂\r\n").unwrap();
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content.text = "keep unsaved".into();
    frame(
        &mut app,
        &ctx,
        egui::RawInput {
            dropped_files: vec![egui::DroppedFile {
                path: Some(path.clone()),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.busy || app.drops_pending() {
        frame(&mut app, &ctx, Default::default());
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(app.docs.len(), 2);
    assert_eq!(app.docs[0].content.text, "keep unsaved");
    assert!(app.docs[0].dirty());
    assert_eq!(app.docs[1].content.text, "中文🙂\n");
    assert!(app.docs[1].content.bom);
    assert_eq!(app.docs[1].content.newline, crate::core::Newline::Crlf);
    assert_eq!(
        std::fs::read(path).unwrap(),
        "\u{feff}中文🙂\r\n".as_bytes()
    );
}
