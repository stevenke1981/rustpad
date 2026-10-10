use crate::{App, app_tests, i18n};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, vec2};

#[test]
fn v13_duplicate_shortcut_duplicates_only_selected_unicode_and_is_one_undo() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content = crate::core::Content {
        text: "外甲🙂尾\n".into(),
        bom: true,
        newline: crate::core::Newline::Crlf,
    };
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    app.docs[0].selected = (1, 3);
    app.docs[0].cursor = 3;
    app.selection = Some((1, 3));
    frame(&mut app, &ctx, Default::default());
    shortcut(&mut app, &ctx, egui::Key::D, Modifiers::CTRL);
    assert_eq!(app.docs[0].content.text, "外甲🙂甲🙂尾\n");
    assert_eq!(app.docs[0].selected, (3, 5));
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, before);
    assert!(!app.docs[0].history.can_undo());
}

#[test]
fn v13_move_shortcuts_keep_unicode_block_selection_and_format() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content = crate::core::Content {
        text: "外\n甲🙂\n乙\n尾".into(),
        bom: true,
        newline: crate::core::Newline::Crlf,
    };
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    app.docs[0].selected = (2, 7);
    app.docs[0].cursor = 7;
    app.selection = Some((2, 7));
    frame(&mut app, &ctx, Default::default());
    shortcut(
        &mut app,
        &ctx,
        egui::Key::ArrowUp,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    assert_eq!(app.docs[0].content.text, "甲🙂\n乙\n外\n尾");
    assert_eq!(app.docs[0].selected, (0, 5));
    assert!(
        ctx.memory(|m| m.has_focus(egui::Id::new(("editor", app.docs[0].id)))),
        "move must keep editor focus"
    );
    shortcut(
        &mut app,
        &ctx,
        egui::Key::ArrowDown,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    assert_eq!(app.docs[0].content, before);
    assert_eq!(app.docs[0].selected, (2, 7));
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content.text, "甲🙂\n乙\n外\n尾");
}

#[test]
fn v13_select_current_line_is_not_dirty_and_ignores_search_focus() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content.text = "a\n甲🙂\n尾".into();
    app.docs[0].saved = app.docs[0].content.clone();
    app.docs[0].selected = (3, 3);
    app.docs[0].cursor = 3;
    app.selection = Some((3, 3));
    frame(&mut app, &ctx, Default::default());
    shortcut(
        &mut app,
        &ctx,
        egui::Key::L,
        Modifiers::CTRL | Modifiers::ALT,
    );
    assert_eq!(app.docs[0].selected, (2, 5));
    assert!(!app.docs[0].dirty());
    assert!(!app.docs[0].history.can_undo());
    app.search = true;
    frame(&mut app, &ctx, Default::default());
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("find-query")));
    shortcut(&mut app, &ctx, egui::Key::D, Modifiers::CTRL);
    assert_eq!(app.docs[0].content.text, "a\n甲🙂\n尾");
}

fn frame(app: &mut App, ctx: &egui::Context, mut input: egui::RawInput) -> egui::FullOutput {
    input
        .screen_rect
        .get_or_insert(Rect::from_min_size(Pos2::ZERO, vec2(1080., 720.)));
    let mut frame = eframe::Frame::_new_kittest();
    eframe::App::raw_input_hook(app, ctx, &mut input);
    ctx.run(input, |ctx| eframe::App::update(app, ctx, &mut frame))
}

#[test]
fn v13_move_menu_keeps_focus_for_undo_and_ime_input_does_not_duplicate() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].content.text = "outer\n中文🙂\ntail".into();
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    app.docs[0].selected = (6, 6);
    app.docs[0].cursor = 6;
    app.selection = Some((6, 6));
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "Edit"));
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "Line operations"));
    let output = frame(&mut app, &ctx, Default::default());
    click(
        &mut app,
        &ctx,
        text_center(&output, "Move selected/current lines up    Ctrl+Shift+↑"),
    );
    assert_eq!(app.docs[0].content.text, "中文🙂\nouter\ntail");
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, before);
    let mut input = egui::RawInput {
        events: vec![
            Event::Ime(egui::ImeEvent::Preedit("字".into())),
            Event::Key {
                key: egui::Key::D,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            },
        ],
        ..Default::default()
    };
    app.selection_input(&ctx, &mut input);
    assert_eq!(input.events.len(), 2);
    assert_eq!(app.docs[0].content, before);
    assert!(!app.docs[0].history.can_undo());
}

fn shortcut(
    app: &mut App,
    ctx: &egui::Context,
    key: egui::Key,
    modifiers: Modifiers,
) -> egui::FullOutput {
    frame(
        app,
        ctx,
        egui::RawInput {
            modifiers,
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        },
    )
}

fn text_shapes(output: &egui::FullOutput) -> Vec<&egui::epaint::TextShape> {
    fn collect<'a>(shape: &'a egui::epaint::Shape, texts: &mut Vec<&'a egui::epaint::TextShape>) {
        match shape {
            egui::epaint::Shape::Text(text) => texts.push(text),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, texts);
                }
            }
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut texts);
    }
    texts
}

#[test]
fn view_menu_wrap_keeps_real_lines_eol_cursor_bytes_and_copied_unicode() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].content = crate::core::Content {
        text: format!(
            "{}中文🙂\n\t{}\ntail",
            "alpha beta gamma ".repeat(16),
            "Z".repeat(180)
        ),
        bom: true,
        newline: crate::core::Newline::Crlf,
    };
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    let bytes = crate::core::encode(&before).unwrap();
    app.show_eol = true;
    app.docs[0].cursor = 8;
    app.docs[0].selected = (8, 8);
    app.selection = Some((8, 8));
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "View"));
    let output = frame(&mut app, &ctx, Default::default());
    click(
        &mut app,
        &ctx,
        text_center(&output, "Word wrap (display only)"),
    );
    assert!(app.view.wrap);
    // Close the menu and inspect the actual TextEdit galley and gutter paint.
    shortcut(&mut app, &ctx, egui::Key::Escape, Modifiers::NONE);
    let output = frame(&mut app, &ctx, Default::default());
    let texts = text_shapes(&output);
    let editor = texts
        .iter()
        .find(|text| text.galley.text() == before.text)
        .unwrap();
    assert!(editor.galley.rows.len() > 3);
    assert_eq!(
        editor
            .galley
            .rows
            .iter()
            .filter(|row| row.ends_with_newline)
            .count(),
        2
    );
    assert!(editor.galley.size().x < 1000.);
    for line in 1..=3 {
        assert_eq!(
            texts
                .iter()
                .filter(|text| text.galley.text() == format!(" {line:>4}"))
                .count(),
            1
        );
    }
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.galley.text() == "EOF")
            .count(),
        1
    );
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.galley.text() == "CRLF"
                && text.pos.y >= editor.pos.y
                && text.pos.y < editor.pos.y + editor.galley.size().y)
            .count(),
        2
    );
    assert_eq!(app.docs[0].cursor, 8);
    assert_eq!(app.docs[0].selected, (8, 8));
    assert_eq!(crate::core::encode(&app.docs[0].content).unwrap(), bytes);
    assert!(!app.docs[0].dirty());
    assert!(!app.docs[0].history.can_undo());
    let end = before.text.chars().count();
    app.docs[0].cursor = end;
    app.selection = Some((0, end));
    frame(&mut app, &ctx, Default::default());
    let output = frame(
        &mut app,
        &ctx,
        egui::RawInput {
            events: vec![Event::Copy],
            ..Default::default()
        },
    );
    assert!(output.platform_output.commands.iter().any(
        |command| matches!(command, egui::OutputCommand::CopyText(text) if text == &before.text)
    ));
    frame(
        &mut app,
        &ctx,
        egui::RawInput {
            events: vec![Event::Paste("新🙂\n".into())],
            ..Default::default()
        },
    );
    assert_eq!(app.docs[0].content.text, "新🙂\n");
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, before);
    assert!(!app.docs[0].history.can_undo());
}

#[test]
fn editor_zoom_keys_and_wheel_preserve_document_and_global_ui_scale() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content.text = "中文🙂\tvalue\n".into();
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    app.selection = Some((2, 2));
    app.docs[0].cursor = 2;
    frame(&mut app, &ctx, Default::default());
    let zoom = ctx.zoom_factor();
    let output = shortcut(
        &mut app,
        &ctx,
        egui::Key::Equals,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    assert_eq!(app.view.font_size, 16.);
    assert_eq!(ctx.zoom_factor(), zoom);
    let texts = text_shapes(&output);
    let editor = texts
        .iter()
        .find(|text| text.galley.text() == before.text)
        .unwrap();
    assert_eq!(editor.galley.job.sections[0].format.font_id.size, 16.);
    let pos = editor.pos + vec2(4., 5.);
    frame(
        &mut app,
        &ctx,
        egui::RawInput {
            modifiers: Modifiers::CTRL,
            events: vec![
                Event::PointerMoved(pos),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: vec2(0., 1.),
                    modifiers: Modifiers::CTRL,
                },
            ],
            ..Default::default()
        },
    );
    assert_eq!(app.view.font_size, 17.);
    assert_eq!(ctx.zoom_factor(), zoom);
    shortcut(&mut app, &ctx, egui::Key::Minus, Modifiers::CTRL);
    assert_eq!(app.view.font_size, 16.);
    for _ in 0..40 {
        shortcut(&mut app, &ctx, egui::Key::Plus, Modifiers::CTRL);
    }
    assert_eq!(app.view.font_size, 32.);
    for _ in 0..40 {
        shortcut(&mut app, &ctx, egui::Key::Minus, Modifiers::CTRL);
    }
    assert_eq!(app.view.font_size, 9.);
    shortcut(&mut app, &ctx, egui::Key::Num0, Modifiers::CTRL);
    assert_eq!(app.view.font_size, 15.);
    app.docs[0].composing = true;
    shortcut(&mut app, &ctx, egui::Key::Plus, Modifiers::CTRL);
    assert_eq!(app.view.font_size, 15.);
    app.docs[0].composing = false;
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("search-field-test")));
    shortcut(&mut app, &ctx, egui::Key::Plus, Modifiers::CTRL);
    assert_eq!(app.view.font_size, 15.);
    assert_eq!(app.docs[0].content, before);
    assert_eq!(app.docs[0].cursor, 2);
    assert!(!app.docs[0].history.can_undo());
}

#[test]
fn wrapped_and_zoomed_fold_gutter_keeps_source_numbers_and_bookmarks() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.docs[0].content.text = format!(
        "fn main() {{ // {}\n    hidden 中文🙂\n    hidden two\n}}\ntail",
        "long header ".repeat(20)
    );
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    let start = before.text.find('\n').unwrap() + 1;
    let end = before.text.rfind("}\n").unwrap();
    app.docs[0].folds.toggle(&before.text, start..end);
    app.docs[0].bookmarks.toggle(&before.text, 4);
    app.view.wrap = true;
    app.view.font_size = 20.;
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    let texts = text_shapes(&output);
    for label in [">   1", "    4", "*   5"] {
        assert_eq!(
            texts
                .iter()
                .filter(|text| text.galley.text() == label)
                .count(),
            1,
            "{label}"
        );
    }
    assert!(
        !texts
            .iter()
            .any(|text| matches!(text.galley.text(), "    2" | "    3"))
    );
    let editor = texts
        .iter()
        .find(|text| text.galley.text() == before.text)
        .unwrap();
    assert!(editor.galley.rows.len() > 5);
    assert_eq!(app.docs[0].content, before);
    assert_eq!(
        app.docs[0].folds.hidden.as_slice(),
        std::slice::from_ref(&(start..end))
    );
}

#[test]
fn comment_shortcut_is_one_undo_and_unsupported_language_leaves_content_intact() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].language = Some(crate::syntax::Language::Rust);
    app.docs[0].content = crate::core::Content {
        text: "outside\n  let 中 = \"🙂\";\n\tprintln!();\n0tail\n".into(),
        bom: true,
        newline: crate::core::Newline::Crlf,
    };
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    let end = crate::actions::goto_line(&before.text, 4).unwrap();
    app.docs[0].selected = (8, end);
    app.docs[0].cursor = end;
    app.selection = Some((8, end));
    frame(&mut app, &ctx, Default::default());
    shortcut(&mut app, &ctx, egui::Key::Q, Modifiers::CTRL);
    assert_eq!(
        app.docs[0].content.text,
        "outside\n  // let 中 = \"🙂\";\n\t// println!();\n0tail\n"
    );
    let commented = app.docs[0].content.clone();
    assert!(app.docs[0].dirty());
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, before);
    assert!(!app.docs[0].history.can_undo());
    shortcut(&mut app, &ctx, egui::Key::Y, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, commented);
    app.docs[0].language = Some(crate::syntax::Language::Json);
    let selection = app.docs[0].selected;
    shortcut(&mut app, &ctx, egui::Key::Q, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, commented);
    assert_eq!(app.docs[0].selected, selection);
    assert!(
        app.locale
            .message(&app.message)
            .contains("No comments defined")
    );
}

#[test]
fn line_operations_menu_sorts_whole_document_and_undo_restores_format() {
    let ctx = egui::Context::default();
    let mut app = app_tests::app();
    app.locale = i18n::Locale::English;
    app.docs[0].content = crate::core::Content {
        text: "z\n中文🙂\na\nz\n".into(),
        bom: true,
        newline: crate::core::Newline::Cr,
    };
    app.docs[0].saved = app.docs[0].content.clone();
    let before = app.docs[0].content.clone();
    app.docs[0].bookmarks.toggle(&before.text, 1);
    frame(&mut app, &ctx, Default::default());
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "Edit"));
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "Line operations"));
    let output = frame(&mut app, &ctx, Default::default());
    click(&mut app, &ctx, text_center(&output, "Sort lines ascending"));
    assert_eq!(app.docs[0].content.text, "a\nz\nz\n中文🙂\n");
    assert!(app.docs[0].bookmarks.lines.is_empty());
    shortcut(&mut app, &ctx, egui::Key::Z, Modifiers::CTRL);
    assert_eq!(app.docs[0].content, before);
    assert!(!app.docs[0].history.can_undo());
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
