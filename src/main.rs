use gtk4 as gtk;
use gtk::gdk;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, CssProvider, FlowBox, Label, ScrolledWindow, SearchEntry,
};

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

fn main() {
    let app = Application::builder()
        .application_id("com.emojiclip.app")
        .build();

    app.add_main_option(
        "stdout",
        glib::Char::from(0u8),
        glib::OptionFlags::empty(),
        glib::OptionArg::None,
        "Print selected emoji to stdout instead of copying",
        None,
    );

    app.add_main_option(
        "cmd",
        glib::Char::from(0u8),
        glib::OptionFlags::empty(),
        glib::OptionArg::String,
        "Run COMMAND with the selected emoji as its last argument",
        Some("COMMAND"),
    );

    app.set_option_context_description(Some(
        "Examples:\n  emojiclip --stdout\n  emojiclip --cmd 'wl-copy -o'",
    ));

    let stdout_mode = Rc::new(Cell::new(false));
    let cmd: Rc<RefCell<Option<Vec<String>>>> = Rc::new(RefCell::new(None));

    {
        let stdout_mode = stdout_mode.clone();
        let cmd = cmd.clone();
        app.connect_handle_local_options(move |_, options| {
            if options.contains("stdout") {
                stdout_mode.set(true);
            }
            if let Ok(Some(raw)) = options.lookup::<String>("cmd") {
                let cmd_vec: Vec<String> = raw.split_whitespace().map(String::from).collect();
                *cmd.borrow_mut() = Some(cmd_vec);
            }
            -1
        });
    }

    {
        let stdout_mode = stdout_mode.clone();
        let cmd = cmd.clone();
        app.connect_activate(move |app| {
            build_ui(app, stdout_mode.get(), cmd.borrow().clone());
        });
    }

    app.run();
}

fn build_ui(app: &Application, stdout_mode: bool, cmd: Option<Vec<String>>) {
    // CSS for emoji sizing
    let css = CssProvider::new();
    css.load_from_data(
        ".emoji-label { font-size: 24px; padding: 0; margin: 0; } \
         flowboxchild { all: unset; } \
         flowboxchild:selected { background: alpha(currentColor, 0.15); } \
         flowbox { padding: 0; margin: 0; } \
         searchentry, searchentry:focus, searchentry:hover, searchentry:focus-within, searchentry:disabled { \
           background: none; border: none; box-shadow: none; outline: none; min-height: 0; \
         } \
         searchentry > text, searchentry > text:focus, searchentry > text:hover { \
           background: none; border: none; box-shadow: none; outline: none; \
           caret-color: currentColor; padding-left: 4px; \
         } \
         searchentry > image { opacity: 0; min-width: 0; min-height: 0; margin: 0; padding: 0; } \
         scrollbar { opacity: 0; }",
    );
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not get default display"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search emojis...")
        .hexpand(true)
        .build();

    let flowbox = FlowBox::builder()
        .homogeneous(true)
        .selection_mode(gtk::SelectionMode::Single)
        .max_children_per_line(20)
        .min_children_per_line(8)
        .row_spacing(0)
        .column_spacing(0)
        .valign(gtk::Align::Start)
        .build();

    // Collect all entries (fast, no GTK widgets yet)
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut added: HashSet<String> = HashSet::new();
    for emoji in emojis::iter() {
        entries.push((emoji.as_str().to_string(), emoji.name().to_string()));
        added.insert(emoji.as_str().to_string());
    }

    const SYMBOL_RANGES: &[(u32, u32)] = &[
        (0x00A1, 0x00BF), // Latin-1 punctuation/symbols
        (0x00D7, 0x00D7), // Multiplication sign
        (0x00F7, 0x00F7), // Division sign
        (0x2010, 0x2027), // General Punctuation
        (0x2030, 0x205E), // General Punctuation cont.
        (0x2070, 0x209F), // Superscripts and Subscripts
        (0x20A0, 0x20CF), // Currency Symbols
        (0x2100, 0x214F), // Letterlike Symbols
        (0x2150, 0x218F), // Number Forms
        (0x2190, 0x21FF), // Arrows
        (0x2200, 0x22FF), // Mathematical Operators
        (0x2300, 0x23FF), // Miscellaneous Technical
        (0x2400, 0x243F), // Control Pictures
        (0x2440, 0x245F), // Optical Character Recognition
        (0x2460, 0x24FF), // Enclosed Alphanumerics
        (0x2500, 0x257F), // Box Drawing
        (0x2580, 0x259F), // Block Elements
        (0x25A0, 0x25FF), // Geometric Shapes
        (0x2600, 0x26FF), // Miscellaneous Symbols
        (0x2700, 0x27BF), // Dingbats
        (0x27C0, 0x27EF), // Misc Mathematical Symbols-A
        (0x27F0, 0x27FF), // Supplemental Arrows-A
        (0x2800, 0x28FF), // Braille Patterns
        (0x2900, 0x297F), // Supplemental Arrows-B
        (0x2980, 0x29FF), // Misc Mathematical Symbols-B
        (0x2A00, 0x2AFF), // Supplemental Mathematical Operators
        (0x2B00, 0x2BFF), // Misc Symbols and Arrows
    ];
    for &(start, end) in SYMBOL_RANGES {
        for cp in start..=end {
            if let Some(c) = char::from_u32(cp) {
                if c.is_control() || c.is_whitespace() {
                    continue;
                }
                let s = c.to_string();
                if added.contains(&s) {
                    continue;
                }
                if let Some(name) = unicode_names2::name(c) {
                    entries.push((s.clone(), name.to_string()));
                    added.insert(s);
                }
            }
        }
    }

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .child(&flowbox)
        .build();

    // Search filtering
    let query: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let query_for_filter = query.clone();

    flowbox.set_filter_func(move |child| matches_filter(child, &query_for_filter.borrow()));

    let flowbox_for_search = flowbox.clone();
    let scrolled_for_search = scrolled.clone();
    let query_for_search = query.clone();
    search_entry.connect_search_changed(move |entry| {
        *query_for_search.borrow_mut() = entry.text().to_lowercase().to_string();
        flowbox_for_search.invalidate_filter();
        select_first_match(&flowbox_for_search, &scrolled_for_search, &query_for_search.borrow());
    });

    // Click/activate to copy and quit
    let app_for_click = app.clone();
    let cmd_for_click = cmd.clone();
    flowbox.connect_child_activated(move |_, child| {
        if let Some(label) = child.child().and_downcast::<Label>() {
            copy_and_quit(&label.text(), &app_for_click, stdout_mode, &cmd_for_click);
        }
    });

    // Enter to copy selected emoji and quit
    let app_for_enter = app.clone();
    let flowbox_for_activate = flowbox.clone();
    search_entry.connect_activate(move |_| {
        if let Some(child) = flowbox_for_activate.selected_children().first() {
            if let Some(label) = child.child().and_downcast::<Label>() {
                copy_and_quit(&label.text(), &app_for_enter, stdout_mode, &cmd);
            }
        }
    });

    // Arrow key navigation
    let key_controller = gtk::EventControllerKey::new();
    let flowbox_for_keys = flowbox.clone();
    let scrolled_for_keys = scrolled.clone();
    let query_for_keys = query.clone();
    let app_for_escape = app.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            app_for_escape.quit();
            return glib::Propagation::Stop;
        }
        let q = query_for_keys.borrow();
        let stepped = match key {
            gdk::Key::Left => 1,
            gdk::Key::Right => 1,
            gdk::Key::Up | gdk::Key::Down => get_columns(&flowbox_for_keys, &q),
            _ => return glib::Propagation::Proceed,
        };
        let direction = match key {
            gdk::Key::Left | gdk::Key::Up => -(stepped as i32),
            _ => stepped as i32,
        };
        move_by(&flowbox_for_keys, &scrolled_for_keys, &q, direction);
        glib::Propagation::Stop
    });
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    search_entry.add_controller(key_controller);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.append(&search_entry);
    vbox.append(&scrolled);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("emojiclip")
        .default_width(500)
        .default_height(400)
        .child(&vbox)
        .build();

    search_entry.set_key_capture_widget(Some(&window));

    window.present();

    // Batch-populate the grid after the window is visible
    let entries = Rc::new(entries);
    let pos = Rc::new(Cell::new(0usize));
    let flowbox_for_pop = flowbox.clone();
    glib::idle_add_local(move || {
        let batch_end = (pos.get() + 200).min(entries.len());
        for i in pos.get()..batch_end {
            let (ref text, ref name) = entries[i];
            flowbox_for_pop.insert(&make_label(text, name), -1);
        }
        if pos.get() == 0 {
            if let Some(first) = flowbox_for_pop.child_at_index(0) {
                flowbox_for_pop.select_child(&first);
            }
        }
        pos.set(batch_end);
        if batch_end >= entries.len() {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn matches_filter(child: &gtk::FlowBoxChild, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    child
        .child()
        .and_downcast::<Label>()
        .and_then(|label| label.tooltip_text())
        .map(|name| sublime_fuzzy::best_match(query, &name.to_lowercase()).is_some())
        .unwrap_or(false)
}

fn copy_and_quit(text: &str, app: &Application, stdout_mode: bool, cmd: &Option<Vec<String>>) {
    if stdout_mode {
        print!("{}", text);
    } else if let Some(cmd_args) = cmd {
        if let Some((program, args)) = cmd_args.split_first() {
            let mut child_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            child_args.push(text);
            let _ = std::process::Command::new(program).args(&child_args).spawn();
        }
    } else if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);
    }
    app.quit();
}

fn select_first_match(flowbox: &FlowBox, scrolled: &ScrolledWindow, query: &str) {
    scrolled.vadjustment().set_value(0.0);
    let mut i = 0;
    while let Some(child) = flowbox.child_at_index(i) {
        if matches_filter(&child, query) {
            flowbox.select_child(&child);
            return;
        }
        i += 1;
    }
}

/// Count how many visible children share the first row's y-coordinate.
fn get_columns(flowbox: &FlowBox, query: &str) -> usize {
    let mut first_y = None;
    let mut cols = 0usize;
    let mut i = 0;
    while let Some(child) = flowbox.child_at_index(i) {
        if matches_filter(&child, query) {
            let y = child.allocation().y();
            match first_y {
                None => {
                    first_y = Some(y);
                    cols = 1;
                }
                Some(fy) if y == fy => cols += 1,
                _ => break,
            }
        }
        i += 1;
    }
    cols.max(1)
}

/// Move selection by `count` visible items (positive = forward, negative = backward).
fn move_by(flowbox: &FlowBox, scrolled: &ScrolledWindow, query: &str, count: i32) {
    let selected = flowbox.selected_children();
    let current_idx = match selected.first() {
        Some(child) => child.index(),
        None => return,
    };

    let direction = if count > 0 { 1 } else { -1 };
    let mut remaining = count.unsigned_abs() as usize;
    let mut target = None;
    let mut i = current_idx + direction;

    while remaining > 0 && i >= 0 {
        match flowbox.child_at_index(i) {
            Some(child) if matches_filter(&child, query) => {
                target = Some(child);
                remaining -= 1;
                i += direction;
            }
            Some(_) => i += direction,
            None => break,
        }
    }

    if let Some(child) = target {
        flowbox.select_child(&child);
        scroll_to_child(scrolled, &child);
    }
}

const CELL_SIZE: i32 = 32;

fn make_label(text: &str, name: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("emoji-label");
    label.set_tooltip_text(Some(name));
    label.set_max_width_chars(1);
    label.set_size_request(CELL_SIZE, CELL_SIZE);
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::Center);

    // Measure rendered size and scale down if it exceeds the cell size in either dimension
    let pango_ctx = label.pango_context();
    let layout = pango::Layout::new(&pango_ctx);
    let mut font = pango::FontDescription::new();
    font.set_absolute_size(CELL_SIZE as f64 * pango::SCALE as f64);
    layout.set_font_description(Some(&font));
    layout.set_text(text);
    let (w, h) = layout.pixel_size();
    let max_dim = w.max(h);
    if max_dim > CELL_SIZE {
        let new_size = CELL_SIZE as f64 * CELL_SIZE as f64 / max_dim as f64;
        let mut small_font = pango::FontDescription::new();
        small_font.set_absolute_size(new_size * pango::SCALE as f64);
        let attrs = pango::AttrList::new();
        attrs.insert(pango::AttrFontDesc::new(&small_font));
        label.set_attributes(Some(&attrs));
    }

    label
}

fn scroll_to_child(scrolled: &ScrolledWindow, child: &impl IsA<gtk::Widget>) {
    let allocation = child.allocation();
    let y = allocation.y() as f64;
    let h = allocation.height() as f64;
    let adj = scrolled.vadjustment();
    if y < adj.value() {
        adj.set_value(y);
    } else if y + h > adj.value() + adj.page_size() {
        adj.set_value(y + h - adj.page_size());
    }
}
