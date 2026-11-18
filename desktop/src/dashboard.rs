use crate::{data, preferences};
use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub struct DashboardWidgets {
    pub window: glib::WeakRef<gtk::ApplicationWindow>,
    pub word_total: gtk::Label,
    pub saved_time: gtk::Label,
    pub dictation_total: gtk::Label,
    pub home_error: gtk::Label,
    pub recent_list: gtk::Box,
    pub search_entry: gtk::SearchEntry,
    pub history_list: gtk::Box,
    pub history_error: gtk::Label,
    pub load_more: gtk::Button,
    pub typing_speed: gtk::SpinButton,
    pub settings_status: gtk::Label,
}

struct Dashboard {
    widgets: DashboardWidgets,
    limit: u32,
    refreshing: bool,
    refresh_requested: bool,
    recent: Option<Vec<data::Dictation>>,
    history: Option<Vec<data::Dictation>>,
    displayed_search: String,
    displayed_totals: CounterValues,
    target_totals: Option<CounterValues>,
    counter_animation: Option<gtk::TickCallbackId>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct CounterValues {
    words: f64,
    seconds_saved: f64,
    dictations: f64,
}

pub fn connect(widgets: DashboardWidgets, home_button: &gtk::ToggleButton) {
    let dashboard = Rc::new(RefCell::new(Dashboard {
        widgets,
        limit: 50,
        refreshing: false,
        refresh_requested: false,
        recent: None,
        history: None,
        displayed_search: String::new(),
        displayed_totals: CounterValues::default(),
        target_totals: None,
        counter_animation: None,
    }));
    let weak = Rc::downgrade(&dashboard);
    home_button.connect_clicked(move |_| {
        if let Some(dashboard) = weak.upgrade() {
            let mut state = dashboard.borrow_mut();
            if let Some(target) = state.target_totals {
                state.target_totals = None;
                render_counters(&mut state, CounterValues::default());
                update_counters(&dashboard, &mut state, target);
            }
        }
    });
    let weak = Rc::downgrade(&dashboard);
    dashboard
        .borrow()
        .widgets
        .search_entry
        .connect_search_changed(move |_| {
            if let Some(dashboard) = weak.upgrade() {
                dashboard.borrow_mut().limit = 50;
                refresh(&dashboard);
            }
        });
    let weak = Rc::downgrade(&dashboard);
    dashboard
        .borrow()
        .widgets
        .load_more
        .connect_clicked(move |_| {
            if let Some(dashboard) = weak.upgrade() {
                let limit = dashboard.borrow().limit.saturating_add(50);
                dashboard.borrow_mut().limit = limit;
                refresh(&dashboard);
            }
        });
    let weak = Rc::downgrade(&dashboard);
    dashboard
        .borrow()
        .widgets
        .typing_speed
        .connect_value_changed(move |spin| {
            if let Some(dashboard) = weak.upgrade() {
                let speed = spin.value_as_int() as u32;
                let state = dashboard.borrow();
                match preferences::save_typing_speed(speed) {
                    Ok(()) => state
                        .widgets
                        .settings_status
                        .set_text("Typing speed saved."),
                    Err(error) => state.widgets.settings_status.set_text(&error),
                }
                drop(state);
                refresh(&dashboard);
            }
        });
    refresh(&dashboard);
    glib::timeout_add_local(Duration::from_secs(2), move || {
        if dashboard.borrow().widgets.window.upgrade().is_none() == true {
            glib::ControlFlow::Break
        } else {
            if dashboard.borrow().refreshing == false {
                refresh(&dashboard);
            }
            glib::ControlFlow::Continue
        }
    });
}

fn refresh(dashboard: &Rc<RefCell<Dashboard>>) {
    let mut state = dashboard.borrow_mut();
    if state.widgets.window.upgrade().is_none() == true {
        return;
    }
    if state.refreshing == true {
        state.refresh_requested = true;
        return;
    }
    state.refreshing = true;
    state.refresh_requested = false;
    let search = state.widgets.search_entry.text().to_string();
    let limit = state.limit;
    drop(state);
    let weak = Rc::downgrade(dashboard);
    glib::spawn_future_local(async move {
        let requested_search = search.clone();
        let result = gio::spawn_blocking(move || {
            (
                data::load_statistics(),
                data::list_dictations("", 5),
                data::list_dictations(&requested_search, limit.saturating_add(1)),
            )
        })
        .await;
        let dashboard = match weak.upgrade() {
            Some(dashboard) => dashboard,
            None => return,
        };
        let mut state = dashboard.borrow_mut();
        state.refreshing = false;
        if state.widgets.window.upgrade().is_none() == true {
            return;
        }
        if state.refresh_requested == true
            || state.widgets.search_entry.text().as_str() != search
            || state.limit != limit
        {
            drop(state);
            refresh(&dashboard);
            return;
        }
        match result {
            Ok((statistics, recent, history)) => {
                let mut home_errors = Vec::new();
                match statistics {
                    Ok(statistics) => {
                        let baseline = state.widgets.typing_speed.value();
                        let totals = CounterValues {
                            words: statistics.word_count as f64,
                            seconds_saved: statistics.estimated_seconds_saved(baseline),
                            dictations: statistics.dictation_count as f64,
                        };
                        update_counters(&dashboard, &mut state, totals);
                        state.widgets.saved_time.set_tooltip_text(Some(&format!("Estimated at {baseline:.0} typed words per minute, minus recording time.")));
                    }
                    Err(error) => {
                        if let Some(animation) = state.counter_animation.take() {
                            animation.remove();
                        }
                        state.target_totals = None;
                        state.widgets.word_total.set_text("—");
                        state.widgets.dictation_total.set_text("—");
                        state.widgets.saved_time.set_text("—");
                        home_errors.push(error);
                    }
                }
                match recent {
                    Ok(records) => {
                        if state.recent.as_ref() != Some(&records) {
                            populate_list(&state.widgets.recent_list, &records, false);
                            state.recent = Some(records);
                        }
                    }
                    Err(error) => home_errors.push(error),
                }
                match history {
                    Ok(mut records) => {
                        state
                            .widgets
                            .load_more
                            .set_visible(records.len() > limit as usize);
                        records.truncate(limit as usize);
                        if state.history.as_ref() != Some(&records)
                            || state.displayed_search != search
                        {
                            populate_list(
                                &state.widgets.history_list,
                                &records,
                                search.is_empty() == false,
                            );
                            state.history = Some(records);
                            state.displayed_search = search;
                        }
                        show_error(&state.widgets.history_error, "");
                    }
                    Err(error) => {
                        show_error(&state.widgets.history_error, &error);
                        state.widgets.load_more.set_visible(false);
                    }
                }
                show_error(&state.widgets.home_error, &home_errors.join("\n"));
            }
            Err(_) => {
                show_error(
                    &state.widgets.home_error,
                    "History could not be loaded. Quill will retry shortly.",
                );
                show_error(
                    &state.widgets.history_error,
                    "History could not be loaded. Quill will retry shortly.",
                );
            }
        }
    });
}

fn update_counters(
    dashboard: &Rc<RefCell<Dashboard>>,
    state: &mut Dashboard,
    target: CounterValues,
) {
    if state.target_totals == Some(target) {
        return;
    }
    if let Some(animation) = state.counter_animation.take() {
        animation.remove();
    }
    state.target_totals = Some(target);
    let window = match state.widgets.window.upgrade() {
        Some(window) => window,
        None => return,
    };
    let frame_clock = match window.frame_clock() {
        Some(frame_clock) => frame_clock,
        None => {
            render_counters(state, target);
            return;
        }
    };
    if window.settings().is_gtk_enable_animations() == false
        || state.widgets.word_total.is_mapped() == false
    {
        render_counters(state, target);
        return;
    }
    let start = state.displayed_totals;
    let start_time = frame_clock.frame_time();
    let weak = Rc::downgrade(dashboard);
    state.counter_animation = Some(window.add_tick_callback(move |window, clock| {
        let dashboard = match weak.upgrade() {
            Some(dashboard) => dashboard,
            None => return glib::ControlFlow::Break,
        };
        let mut state = dashboard.borrow_mut();
        let progress = (clock.frame_time() - start_time) as f64 / 700_000.0;
        if progress >= 1.0 || window.settings().is_gtk_enable_animations() == false {
            render_counters(&mut state, target);
            state.counter_animation = None;
            glib::ControlFlow::Break
        } else {
            let values = CounterValues {
                words: interpolate_counter(start.words, target.words, progress),
                seconds_saved: interpolate_counter(
                    start.seconds_saved,
                    target.seconds_saved,
                    progress,
                ),
                dictations: interpolate_counter(start.dictations, target.dictations, progress),
            };
            render_counters(&mut state, values);
            glib::ControlFlow::Continue
        }
    }));
}

fn interpolate_counter(start: f64, target: f64, progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    let eased_progress = 1.0 - (1.0 - progress).powi(3);
    start + (target - start) * eased_progress
}

fn render_counters(state: &mut Dashboard, values: CounterValues) {
    state.displayed_totals = values;
    state
        .widgets
        .word_total
        .set_text(&format!("{:.0}", values.words));
    state
        .widgets
        .saved_time
        .set_text(&format_saved_time(values.seconds_saved));
    state
        .widgets
        .dictation_total
        .set_text(&format!("{:.0}", values.dictations));
}

fn show_error(label: &gtk::Label, message: &str) {
    label.set_text(message);
    label.set_visible(message.is_empty() == false);
}

fn format_saved_time(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.0} sec")
    } else if seconds < 3600.0 {
        format!("{:.0} min", seconds / 60.0)
    } else {
        format!("{:.1} hr", seconds / 3600.0)
    }
}

fn populate_list(container: &gtk::Box, records: &[data::Dictation], searching: bool) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    if records.is_empty() == true {
        if searching == true {
            container.append(&crate::text_label(
                "No dictations match your search.",
                "body-copy",
            ));
        } else {
            container.append(&crate::empty_history());
        }
        return;
    }
    for record in records {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 20);
        row.add_css_class("history-row");
        let transcript = crate::text_label(&record.text, "history-text");
        transcript.set_hexpand(true);
        transcript.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        row.append(&transcript);
        let copy = gtk::Button::new();
        copy.add_css_class("flat");
        copy.set_valign(gtk::Align::Center);
        copy.set_tooltip_text(Some("Copy dictation"));
        copy.update_property(&[gtk::accessible::Property::Label("Copy dictation")]);
        copy.set_child(Some(&crate::app_icon("copy", 18)));
        let text = record.text.clone();
        copy.connect_clicked(move |button| {
            button.clipboard().set_text(&text);
            button.set_label("Copied");
            button.set_tooltip_text(Some("Copied"));
            button.update_property(&[gtk::accessible::Property::Label("Copied")]);
            let weak_button = button.downgrade();
            glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                if let Some(button) = weak_button.upgrade() {
                    button.set_child(Some(&crate::app_icon("copy", 18)));
                    button.set_tooltip_text(Some("Copy dictation"));
                    button.update_property(&[gtk::accessible::Property::Label("Copy dictation")]);
                }
            });
        });
        row.append(&copy);
        container.append(&row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_animation_clamps_progress_and_reaches_the_exact_total() {
        assert_eq!(interpolate_counter(20.0, 100.0, -1.0), 20.0);
        assert_eq!(interpolate_counter(20.0, 100.0, 0.0), 20.0);
        assert_eq!(interpolate_counter(20.0, 100.0, 0.5), 90.0);
        assert_eq!(interpolate_counter(20.0, 100.0, 1.0), 100.0);
        assert_eq!(interpolate_counter(20.0, 100.0, 2.0), 100.0);
        assert_eq!(interpolate_counter(100.0, 20.0, 0.5), 30.0);
    }

    #[test]
    fn formats_saved_time_for_dashboard_units() {
        assert_eq!(format_saved_time(0.0), "0 sec");
        assert_eq!(format_saved_time(45.0), "45 sec");
        assert_eq!(format_saved_time(120.0), "2 min");
        assert_eq!(format_saved_time(5400.0), "1.5 hr");
    }
}
