use fancy_window::model::{Point, Rect};
use fancy_window::view::{StatusHit, status_layout};

#[test]
fn segments_from_the_left_version_and_help_on_the_right() {
    let l = status_layout(1000.0, 700.0, 1.0, &[80.0, 100.0], Some(120.0), 40.0);
    assert_eq!(l.bar, Rect::new(0.0, 676.0, 1000.0, 24.0));
    // Each segment is its content plus 9 px a side.
    assert_eq!(l.segments, vec![Rect::new(0.0, 676.0, 98.0, 24.0), Rect::new(98.0, 676.0, 118.0, 24.0)]);
    assert_eq!(l.version, Rect::new(950.0, 676.0, 40.0, 24.0));
    assert_eq!(l.help, Rect::new(920.0, 676.0, 24.0, 24.0));
    assert_eq!(l.hint, Some(Rect::new(782.0, 676.0, 138.0, 24.0)));
}

#[test]
fn the_hint_is_dropped_before_it_overlaps_a_segment() {
    let l = status_layout(400.0, 700.0, 1.0, &[150.0, 100.0], Some(120.0), 40.0);
    assert_eq!(l.hint, None);
}

#[test]
fn hit_testing() {
    let l = status_layout(1000.0, 700.0, 1.0, &[80.0, 100.0], None, 40.0);
    assert_eq!(l.hit(Point::new(10.0, 690.0)), Some(StatusHit::Segment(0)));
    assert_eq!(l.hit(Point::new(150.0, 690.0)), Some(StatusHit::Segment(1)));
    assert_eq!(l.hit(Point::new(930.0, 690.0)), Some(StatusHit::Help));
    assert_eq!(l.hit(Point::new(600.0, 690.0)), None);
    assert_eq!(l.hit(Point::new(10.0, 600.0)), None);
}

#[test]
fn scales_with_dpi() {
    let l = status_layout(1500.0, 1050.0, 1.5, &[80.0], None, 60.0);
    assert_eq!(l.bar.height, 36.0);
    assert_eq!(l.segments[0].width, 80.0 + 2.0 * 14.0);
}
