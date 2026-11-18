use gtk::prelude::*;
use gtk::{DrawingArea, Overlay, Stack, ToggleButton, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SPRAY_SECONDS: f64 = 0.56;
const RIBBON_SECONDS: f64 = 0.84;
const RIBBONS_PER_SECOND: f64 = 120.0;
const RIBBONS_PER_SIDE: u32 = (SPRAY_SECONDS * RIBBONS_PER_SECOND) as u32 + 1;
const DURATION_SECONDS: f64 = SPRAY_SECONDS + RIBBON_SECONDS;

pub fn attach(overlay: &Overlay, home_button: &ToggleButton, pages: &Stack) {
    let ribbons = DrawingArea::new();
    ribbons.set_can_target(false);
    ribbons.set_focusable(false);
    let elapsed = Rc::new(Cell::new(DURATION_SECONDS));
    let animation = Rc::new(RefCell::new(None));
    let draw_elapsed = Rc::clone(&elapsed);
    ribbons.set_draw_func(move |_, context, width, height| {
        draw_ribbons(context, width as f64, height as f64, draw_elapsed.get());
    });
    let map_elapsed = Rc::clone(&elapsed);
    let map_animation = Rc::clone(&animation);
    ribbons.connect_map(move |ribbons| {
        start(ribbons, &map_elapsed, &map_animation);
    });
    let ribbons_reference = ribbons.downgrade();
    home_button.connect_clicked(move |_| {
        if let Some(ribbons) = ribbons_reference.upgrade() {
            start(&ribbons, &elapsed, &animation);
        }
    });
    let ribbons_reference = ribbons.downgrade();
    pages.connect_visible_child_name_notify(move |pages| {
        if let Some(ribbons) = ribbons_reference.upgrade() {
            ribbons.set_visible(pages.visible_child_name().as_deref() == Some("home"));
        }
    });
    overlay.add_overlay(&ribbons);
    overlay.set_clip_overlay(&ribbons, true);
}

fn start(
    ribbons: &DrawingArea,
    elapsed: &Rc<Cell<f64>>,
    animation: &Rc<RefCell<Option<gtk::TickCallbackId>>>,
) {
    if let Some(previous) = animation.borrow_mut().take() {
        previous.remove();
    }
    if ribbons.settings().is_gtk_enable_animations() == false {
        elapsed.set(DURATION_SECONDS);
        ribbons.queue_draw();
        return;
    }
    let clock = match ribbons.frame_clock() {
        Some(clock) => clock,
        None => return,
    };
    let started_at = clock.frame_time();
    elapsed.set(0.0);
    let elapsed = Rc::clone(elapsed);
    let current_animation = Rc::clone(animation);
    *animation.borrow_mut() = Some(ribbons.add_tick_callback(move |ribbons, clock| {
        let seconds = (clock.frame_time() - started_at) as f64 / 1_000_000.0;
        if seconds >= DURATION_SECONDS
            || ribbons.settings().is_gtk_enable_animations() == false
            || ribbons.is_mapped() == false
        {
            elapsed.set(DURATION_SECONDS);
            *current_animation.borrow_mut() = None;
            ribbons.queue_draw();
            glib::ControlFlow::Break
        } else {
            elapsed.set(seconds);
            ribbons.queue_draw();
            glib::ControlFlow::Continue
        }
    }));
}

fn ribbon_position(
    index: u32,
    from_right: bool,
    seconds: f64,
    width: f64,
    height: f64,
) -> (f64, f64) {
    let angle = (36.0 + (index % 7) as f64 * 3.0).to_radians();
    let speed = width.min(height) * (0.8 + (index * 7 % 11) as f64 * 0.035);
    let travel = speed * angle.cos() * seconds;
    let x = if from_right == true {
        width - 24.0 - travel
    } else {
        24.0 + travel
    };
    let gravity = height * 0.72;
    let y = height * 0.78 - speed * angle.sin() * seconds + 0.5 * gravity * seconds * seconds;
    (x, y)
}

fn draw_ribbons(context: &gtk::cairo::Context, width: f64, height: f64, seconds: f64) {
    if seconds >= DURATION_SECONDS {
        return;
    }
    for from_right in [false, true] {
        for index in 0..RIBBONS_PER_SIDE {
            let age = seconds - launch_delay(index);
            if age < 0.0 || age >= RIBBON_SECONDS {
                continue;
            }
            let opacity = ((RIBBON_SECONDS - age) / 0.5).clamp(0.0, 1.0);
            let (x, y) = ribbon_position(index, from_right, age, width, height);
            if context.save().is_err() == true {
                return;
            }
            context.translate(x, y);
            context.rotate(index as f64 * 1.7 + age * (3.0 + (index % 3) as f64));
            match index % 6 {
                0 => context.set_source_rgba(1.0, 0.30, 0.43, opacity),
                1 => context.set_source_rgba(1.0, 0.62, 0.11, opacity),
                2 => context.set_source_rgba(1.0, 0.84, 0.04, opacity),
                3 => context.set_source_rgba(0.18, 0.78, 0.33, opacity),
                4 => context.set_source_rgba(0.23, 0.53, 1.0, opacity),
                _ => context.set_source_rgba(0.66, 0.33, 0.97, opacity),
            }
            let length = 12.0 + (index % 4) as f64 * 3.0;
            context.move_to(-2.0, -length / 2.0);
            context.curve_to(5.0, -length / 4.0, -5.0, length / 4.0, 2.0, length / 2.0);
            context.line_to(6.0, length / 2.0);
            context.curve_to(-1.0, length / 4.0, 9.0, -length / 4.0, 2.0, -length / 2.0);
            context.close_path();
            let painted = context.fill();
            let restored = context.restore();
            if painted.is_err() == true || restored.is_err() == true {
                return;
            }
        }
    }
}

fn launch_delay(index: u32) -> f64 {
    index as f64 / RIBBONS_PER_SECOND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorter_effects_do_not_speed_up_a_ribbons_flight() {
        let position = ribbon_position(3, false, 0.2, 1000.0, 900.0);
        assert!((position.0 - 170.37110370561535).abs() < 0.001);
        assert!((position.1 - 568.5888962943846).abs() < 0.001);
    }

    #[test]
    fn emission_is_spread_across_the_whole_spray() {
        assert_eq!(launch_delay(0), 0.0);
        assert!(launch_delay(RIBBONS_PER_SIDE / 2) >= SPRAY_SECONDS / 2.0);
        let final_launch = launch_delay(RIBBONS_PER_SIDE - 1);
        assert!(final_launch <= SPRAY_SECONDS);
        assert!(final_launch > SPRAY_SECONDS - 1.0 / RIBBONS_PER_SECOND);
        for index in 1..RIBBONS_PER_SIDE {
            let interval = launch_delay(index) - launch_delay(index - 1);
            assert!((interval - 1.0 / 120.0).abs() < 0.0001);
        }
    }

    #[test]
    fn ribbons_spray_inward_from_both_sides_then_fall() {
        let left = ribbon_position(3, false, 0.2, 1000.0, 900.0);
        let right = ribbon_position(3, true, 0.2, 1000.0, 900.0);
        assert!((left.0 + right.0 - 1000.0).abs() < 0.001);
        assert_eq!(left.1, right.1);
        assert!(left.0 > 0.0);
        assert!(left.1 < 900.0 * 0.78);
        let before_end = ribbon_position(3, false, 1.7, 1000.0, 900.0);
        let fallen = ribbon_position(3, false, 1.8, 1000.0, 900.0);
        assert!(fallen.1 > before_end.1);
    }
}
