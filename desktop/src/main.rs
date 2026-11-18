mod celebration;
mod dashboard;
mod data;
mod preferences;

use dashboard::DashboardWidgets;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation, gdk, gio, glib};
use pango::prelude::FontMapExt;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

fn main() -> glib::ExitCode {
    let application = Application::builder()
        .application_id("io.github.tk_hpc.Quill")
        .build();
    application.connect_activate(build_window);
    application.run()
}

fn build_window(application: &Application) {
    if let Some(window) = application.active_window() {
        window.present();
        return;
    }

    let stylesheet = gtk::CssProvider::new();
    stylesheet.load_from_string(include_str!("../style.css"));
    let display = gdk::Display::default().expect("A desktop display is required to open Quill");
    gtk::style_context_add_provider_for_display(
        &display,
        &stylesheet,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let window = ApplicationWindow::builder()
        .application(application)
        .title("Quill")
        .decorated(false)
        .default_width(1240)
        .default_height(850)
        .build();
    window.set_size_request(960, 680);

    match window.pango_context().font_map() {
        Some(font_map) => {
            for filename in ["DM-Sans.ttf", "Newsreader.ttf", "DM-Mono.ttf"] {
                let font_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("assets/fonts")
                    .join(filename);
                match font_map.add_font_file(font_path) {
                    Ok(()) => {}
                    Err(error) => eprintln!("Quill could not load {filename}: {error}"),
                }
            }
        }
        None => eprintln!("Quill could not access the desktop font map"),
    }

    let layout = gtk::Box::new(Orientation::Horizontal, 0);
    layout.add_css_class("app-layout");
    layout.set_overflow(gtk::Overflow::Hidden);
    let sidebar = gtk::Box::new(Orientation::Vertical, 8);
    sidebar.set_width_request(232);
    sidebar.add_css_class("sidebar");
    let wordmark_handle = gtk::WindowHandle::new();
    let brand = gtk::Box::new(Orientation::Horizontal, 10);
    brand.add_css_class("brand-lockup");
    let brand_mark = gtk::Image::from_file(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/brand/quill-mark.png"),
    );
    brand_mark.set_pixel_size(42);
    brand.append(&brand_mark);
    brand.append(&text_label("Quill", "wordmark"));
    wordmark_handle.set_child(Some(&brand));
    sidebar.append(&wordmark_handle);

    let pages = gtk::Stack::new();
    pages.set_hexpand(true);
    pages.set_vexpand(true);
    pages.set_transition_type(gtk::StackTransitionType::Crossfade);
    pages.set_transition_duration(180);
    let widgets = dashboard_widgets(&window);
    pages.add_named(&scrollable_page(&build_home(&widgets)), Some("home"));
    pages.add_named(&scrollable_page(&build_history(&widgets)), Some("history"));
    pages.add_named(
        &scrollable_page(&build_dictionary(&window)),
        Some("dictionary"),
    );
    pages.add_named(
        &scrollable_page(&build_settings(&widgets)),
        Some("settings"),
    );

    let home_button = navigation_button("Home", "home", "home", &pages);
    let history_button = navigation_button("History", "history", "history", &pages);
    let dictionary_button = navigation_button("Dictionary", "dictionary", "dictionary", &pages);
    let settings_button = navigation_button("Settings", "settings", "settings", &pages);
    history_button.set_group(Some(&home_button));
    dictionary_button.set_group(Some(&home_button));
    settings_button.set_group(Some(&home_button));
    home_button.set_active(true);
    sidebar.append(&home_button);
    sidebar.append(&history_button);
    sidebar.append(&dictionary_button);

    let sidebar_space = gtk::Box::new(Orientation::Vertical, 0);
    sidebar_space.set_vexpand(true);
    sidebar.append(&sidebar_space);
    sidebar.append(&settings_button);

    layout.append(&sidebar);
    layout.append(&pages);
    let window_controls = gtk::Button::new();
    window_controls.set_child(Some(&app_icon("close", 20)));
    window_controls.set_tooltip_text(Some("Close Quill"));
    window_controls.add_css_class("window-close");
    let window_reference = window.downgrade();
    window_controls.connect_clicked(move |_| {
        if let Some(window) = window_reference.upgrade() {
            window.close();
        }
    });
    window_controls.set_halign(gtk::Align::End);
    window_controls.set_valign(gtk::Align::Start);
    window_controls.set_margin_top(12);
    window_controls.set_margin_end(12);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&layout));
    celebration::attach(&overlay, &home_button, &pages);
    overlay.add_overlay(&window_controls);
    window.set_child(Some(&overlay));
    window.present();
    home_button.grab_focus();
    dashboard::connect(widgets, &home_button);
}

fn dashboard_widgets(window: &ApplicationWindow) -> DashboardWidgets {
    let home_error = text_label("", "error-message");
    home_error.set_visible(false);
    let history_error = text_label("", "error-message");
    history_error.set_visible(false);
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search your dictations"));
    search_entry.update_property(&[gtk::accessible::Property::Label("Search your dictations")]);
    let load_more = gtk::Button::with_label("Load more");
    load_more.add_css_class("outline-button");
    load_more.set_halign(gtk::Align::Start);
    load_more.set_visible(false);
    let typing_speed = gtk::SpinButton::with_range(10.0, 200.0, 1.0);
    typing_speed.set_value(40.0);
    typing_speed.update_property(&[gtk::accessible::Property::Label(
        "Typing speed in words per minute",
    )]);
    let settings_status = text_label("", "caption");
    match preferences::load_typing_speed() {
        Ok(speed) => typing_speed.set_value(speed as f64),
        Err(error) => {
            settings_status.set_text(&format!("{error} Using 40 WPM until you save a new value."))
        }
    }
    DashboardWidgets {
        window: window.downgrade(),
        word_total: text_label("—", "statistic-value"),
        saved_time: text_label("—", "statistic-value"),
        dictation_total: text_label("—", "statistic-value"),
        home_error,
        recent_list: gtk::Box::new(Orientation::Vertical, 0),
        search_entry,
        history_list: gtk::Box::new(Orientation::Vertical, 0),
        history_error,
        load_more,
        typing_speed,
        settings_status,
    }
}

fn text_label(text: &str, class_name: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class(class_name);
    label.set_wrap(true);
    label
}

fn app_icon(name: &str, size: i32) -> gtk::Image {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/icons")
        .join(format!("{name}.svg"));
    let icon = gtk::Image::from_file(path);
    icon.set_pixel_size(size);
    icon
}

fn page_container() -> gtk::Box {
    let container = gtk::Box::new(Orientation::Vertical, 28);
    container.add_css_class("page");
    container
}

fn scrollable_page(content: &gtk::Box) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(content)
        .build()
}

fn navigation_button(
    title: &str,
    icon_name: &str,
    page_name: &str,
    pages: &gtk::Stack,
) -> gtk::ToggleButton {
    let content = gtk::Box::new(Orientation::Horizontal, 12);
    let icon = app_icon(icon_name, 24);
    content.append(&icon);
    content.append(&text_label(title, "body-copy"));
    let button = gtk::ToggleButton::new();
    button.set_child(Some(&content));
    button.add_css_class("navigation");
    button.update_property(&[gtk::accessible::Property::Label(title)]);
    let pages = pages.clone();
    let page_name = page_name.to_string();
    button.connect_toggled(move |button| {
        if button.is_active() == true {
            pages.set_visible_child_name(&page_name);
        }
    });
    button
}

fn build_home(widgets: &DashboardWidgets) -> gtk::Box {
    let page = page_container();
    page.append(&text_label("Welcome back.", "page-title"));
    let shortcut_panel = gtk::Box::new(Orientation::Vertical, 20);
    shortcut_panel.add_css_class("shortcut-panel");
    shortcut_panel.append(&text_label("Speak. Quill will type.", "display-title"));
    shortcut_panel.append(&text_label(
        "Speak naturally. Your words are transcribed on this device.",
        "body-copy",
    ));
    let shortcut_row = gtk::Box::new(Orientation::Horizontal, 12);
    shortcut_row.set_margin_top(8);
    shortcut_row.append(&text_label("Win", "keycap"));
    shortcut_row.append(&text_label("+", "body-copy"));
    shortcut_row.append(&text_label("X", "keycap"));
    let shortcut_instruction = text_label("Press to speak. Press again to finish.", "body-copy");
    shortcut_instruction.set_margin_start(8);
    shortcut_row.append(&shortcut_instruction);
    shortcut_panel.append(&shortcut_row);
    page.append(&shortcut_panel);

    let statistics = gtk::Box::new(Orientation::Horizontal, 32);
    statistics.add_css_class("statistics");
    statistics.append(&statistic(&widgets.word_total, "Words you didn’t type"));
    statistics.append(&statistic(&widgets.saved_time, "Estimated time saved"));
    statistics.append(&statistic(&widgets.dictation_total, "Dictations"));
    page.append(&statistics);
    page.append(&widgets.home_error);

    let recent_header = gtk::Box::new(Orientation::Horizontal, 12);
    let recent_title = text_label("Recent dictations", "section-title");
    recent_title.set_hexpand(true);
    recent_header.append(&recent_title);
    page.append(&recent_header);
    page.append(&widgets.recent_list);
    page
}

fn statistic(value: &gtk::Label, title: &str) -> gtk::Box {
    let statistic = gtk::Box::new(Orientation::Vertical, 6);
    statistic.set_hexpand(true);
    statistic.append(value);
    statistic.append(&text_label(title, "caption"));
    statistic
}

fn empty_history() -> gtk::Box {
    let empty = gtk::Box::new(Orientation::Vertical, 12);
    empty.add_css_class("empty-state");
    let icon = app_icon("microphone", 32);
    icon.set_halign(gtk::Align::Start);
    empty.append(&icon);
    empty.append(&text_label("Your next thought starts here.", "empty-title"));
    empty.append(&text_label(
        "Use Win + X in any app. Your words will appear here after you finish.",
        "body-copy",
    ));
    empty
}

fn build_history(widgets: &DashboardWidgets) -> gtk::Box {
    let page = page_container();
    page.append(&text_label("History", "page-title"));
    page.append(&text_label(
        "Your words, ready when you need them.",
        "body-copy",
    ));
    page.append(&widgets.search_entry);
    page.append(&widgets.history_error);
    page.append(&widgets.history_list);
    page.append(&widgets.load_more);
    page
}

fn build_dictionary(window: &ApplicationWindow) -> gtk::Box {
    let page = page_container();
    page.append(&text_label("Dictionary", "page-title"));
    page.append(&text_label(
        "A little context goes a long way.",
        "display-title",
    ));
    page.append(&text_label(
        "Names, technical terms, and words you use often. Quill gives these to your model as hints.",
        "body-copy",
    ));
    page.append(&text_label("One word or phrase per line", "section-title"));
    let editor = gtk::TextView::new();
    editor.add_css_class("dictionary-editor");
    editor.set_wrap_mode(gtk::WrapMode::WordChar);
    editor.set_height_request(300);
    editor.update_property(&[gtk::accessible::Property::Label(
        "Custom vocabulary, one term per line",
    )]);
    let editor_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(300)
        .max_content_height(440)
        .propagate_natural_height(true)
        .child(&editor)
        .build();
    page.append(&editor_scroll);
    let actions = gtk::Box::new(Orientation::Horizontal, 12);
    let save = gtk::Button::with_label("Save dictionary");
    save.add_css_class("outline-button");
    let reload = gtk::Button::with_label("Reload from file");
    actions.append(&save);
    actions.append(&reload);
    page.append(&actions);
    let status = text_label("", "caption");
    status.set_selectable(true);
    page.append(&status);

    let etag = Rc::new(RefCell::new(None));
    load_dictionary(&editor, &etag, &status, &save);
    let save_reference = save.downgrade();
    editor.buffer().connect_modified_changed(move |buffer| {
        if let Some(save) = save_reference.upgrade() {
            save.set_sensitive(buffer.is_modified() == true);
        }
    });
    let buffer = editor.buffer();
    let save_etag = Rc::clone(&etag);
    let save_status = status.clone();
    save.connect_clicked(move |_| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        let result = preferences::save_vocabulary(&contents, save_etag.borrow().as_deref());
        match result {
            Ok(updated_etag) => {
                *save_etag.borrow_mut() = updated_etag;
                buffer.set_modified(false);
                save_status.set_text("Saved. These hints apply to your next dictation.");
            }
            Err(error) => save_status.set_text(&error),
        }
    });
    let window_reference = window.downgrade();
    let reload_editor = editor.clone();
    reload.connect_clicked(move |_| {
        let editor = reload_editor.clone();
        let etag = Rc::clone(&etag);
        let status = status.clone();
        let save = save.clone();
        if editor.buffer().is_modified() == false {
            load_dictionary(&editor, &etag, &status, &save);
        } else if let Some(window) = window_reference.upgrade() {
            let dialog = gtk::AlertDialog::builder()
                .message("Discard unsaved dictionary edits?")
                .detail(
                    "Reloading replaces this editor with the vocabulary currently saved on disk.",
                )
                .buttons(["Keep editing", "Reload"])
                .cancel_button(0)
                .default_button(0)
                .build();
            dialog.choose(Some(&window), gio::Cancellable::NONE, move |response| {
                if response == Ok(1) {
                    load_dictionary(&editor, &etag, &status, &save);
                }
            });
        }
    });
    let buffer = editor.buffer();
    window.connect_close_request(move |window| {
        if buffer.is_modified() == false {
            glib::Propagation::Proceed
        } else {
            let dialog = gtk::AlertDialog::builder()
                .message("Close without saving your dictionary?")
                .buttons(["Keep editing", "Discard edits"])
                .cancel_button(0)
                .default_button(0)
                .build();
            let window_reference = window.downgrade();
            let buffer = buffer.clone();
            dialog.choose(Some(window), gio::Cancellable::NONE, move |response| {
                if response == Ok(1) {
                    buffer.set_modified(false);
                    if let Some(window) = window_reference.upgrade() {
                        window.close();
                    }
                }
            });
            glib::Propagation::Stop
        }
    });
    page
}

fn load_dictionary(
    editor: &gtk::TextView,
    etag: &Rc<RefCell<Option<String>>>,
    status: &gtk::Label,
    save: &gtk::Button,
) {
    match preferences::load_vocabulary() {
        Ok((contents, current_etag)) => {
            editor.buffer().set_text(&contents);
            editor.buffer().set_modified(false);
            editor.set_editable(true);
            *etag.borrow_mut() = current_etag;
            status.set_text("Changes apply to your next dictation. Vocabulary hints are not guaranteed replacements.");
        }
        Err(error) => {
            editor.set_editable(false);
            status.set_text(&error);
        }
    }
    save.set_sensitive(false);
}

fn build_settings(widgets: &DashboardWidgets) -> gtk::Box {
    let page = page_container();
    page.append(&text_label("Settings", "page-title"));
    page.append(&text_label("A small app. Your own setup.", "body-copy"));
    let speed_row = gtk::Box::new(Orientation::Vertical, 12);
    speed_row.add_css_class("setting-row");
    speed_row.append(&text_label("Your typing speed", "section-title"));
    speed_row.append(&text_label(
        "Estimates typing time minus recording time. Choose a realistic typing speed for yourself.",
        "body-copy",
    ));
    let speed_control = gtk::Box::new(Orientation::Horizontal, 12);
    speed_control.append(&widgets.typing_speed);
    speed_control.append(&text_label("words per minute", "body-copy"));
    speed_row.append(&speed_control);
    speed_row.append(&widgets.settings_status);
    page.append(&speed_row);
    for (title, description) in [
        ("Shortcut", "Win + X starts and stops dictation."),
        (
            "Inference",
            "Runs locally. CUDA when available, CPU when needed.",
        ),
        (
            "Your data",
            "History stays on this device. No account or subscription.",
        ),
    ] {
        let row = gtk::Box::new(Orientation::Vertical, 8);
        row.add_css_class("setting-row");
        row.append(&text_label(title, "section-title"));
        row.append(&text_label(description, "body-copy"));
        page.append(&row);
    }
    let history_location = match data::database_path() {
        Ok(path) => path.display().to_string(),
        Err(error) => error,
    };
    let data_location = text_label(&history_location, "caption");
    data_location.set_selectable(true);
    data_location.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    page.append(&data_location);
    page.append(&text_label(
        "Tracking starts with new dictations. Your history and totals stay on this device.",
        "caption",
    ));
    page
}
