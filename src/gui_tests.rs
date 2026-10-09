use crate::{App, app_tests, i18n};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, vec2};

fn frame(app: &mut App, ctx: &egui::Context, mut input: egui::RawInput) -> egui::FullOutput {
    input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(1080., 720.)));
    let mut frame = eframe::Frame::_new_kittest();
    eframe::App::raw_input_hook(app, ctx, &mut input);
    ctx.run(input, |ctx| eframe::App::update(app, ctx, &mut frame))
}
fn text_center(output: &egui::FullOutput, text: &str) -> Pos2 {
    fn find(shape: &egui::epaint::Shape, text: &str) -> Option<Pos2> {
        match shape {
            egui::epaint::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.pos + shape.galley.size() / 2.)
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, text)),
            _ => None,
        }
    }
    for clipped in &output.shapes {
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
