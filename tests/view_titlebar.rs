use fancy_window::model::{Point, Rect};
use fancy_window::view::{CaptionButton, TITLE_BAR_HEIGHT, TitleHit, title_layout};

/// File, Layout, Workspaces, Help at roughly their 13 px text widths.
const MENUS: [f64; 4] = [22.0, 40.0, 72.0, 28.0];

#[test]
fn icon_then_menus_from_the_left() {
    let l = title_layout(1200.0, 1.0, &MENUS);
    assert_eq!(l.bar, Rect::new(0.0, 0.0, 1200.0, TITLE_BAR_HEIGHT));
    assert_eq!(l.icon, Rect::new(12.0, 10.0, 16.0, 16.0));
    // Icon ends at 28, then a 6 px gap; each menu is its text plus 8 px a side.
    assert_eq!(l.menus[0], Rect::new(34.0, 0.0, 38.0, 36.0));
    assert_eq!(l.menus[1].x, 72.0);
    assert_eq!(l.menus.len(), 4);
}

#[test]
fn caption_buttons_are_46_wide_at_the_right() {
    let l = title_layout(1200.0, 1.0, &MENUS);
    assert_eq!(l.buttons[0], (CaptionButton::Minimize, Rect::new(1062.0, 0.0, 46.0, 36.0)));
    assert_eq!(l.buttons[1], (CaptionButton::Maximize, Rect::new(1108.0, 0.0, 46.0, 36.0)));
    assert_eq!(l.buttons[2], (CaptionButton::Close, Rect::new(1154.0, 0.0, 46.0, 36.0)));
}

#[test]
fn centre_box_is_centred_and_shrinks_then_disappears() {
    let wide = title_layout(1200.0, 1.0, &MENUS);
    assert_eq!(wide.centre, Some(Rect::new(420.0, 6.0, 360.0, 24.0)));
    // Narrow: squeezed between the menus (+16) and the buttons (-16).
    let narrow = title_layout(600.0, 1.0, &MENUS);
    let c = narrow.centre.expect("still fits");
    assert!(c.x >= narrow.menus[3].right() + 16.0 && c.right() <= narrow.buttons[0].1.x - 16.0);
    assert!(title_layout(460.0, 1.0, &MENUS).centre.is_none());
}

#[test]
fn scales_with_dpi() {
    let l = title_layout(1800.0, 1.5, &MENUS);
    assert_eq!(l.bar.height, 54.0);
    assert_eq!(l.buttons[2].1, Rect::new(1800.0 - 69.0, 0.0, 69.0, 54.0));
}

#[test]
fn hit_testing() {
    let l = title_layout(1200.0, 1.0, &MENUS);
    let hit = |x: f64, y: f64| l.hit(Point::new(x, y), 6.0, false);
    assert_eq!(hit(20.0, 16.0), Some(TitleHit::SystemMenu));
    assert_eq!(hit(50.0, 16.0), Some(TitleHit::Menu(0)));
    // Between the menus and the centre box is caption; the box itself opens the palette.
    assert_eq!(hit(300.0, 16.0), Some(TitleHit::Caption));
    assert_eq!(hit(600.0, 16.0), Some(TitleHit::Centre));
    // Above and below the box (24 px tall in a 36 px row) is still caption.
    assert_eq!(hit(600.0, 33.0), Some(TitleHit::Caption));
    assert_eq!(hit(1120.0, 16.0), Some(TitleHit::Button(CaptionButton::Maximize)));
    assert_eq!(hit(1190.0, 16.0), Some(TitleHit::Button(CaptionButton::Close)));
    assert_eq!(hit(600.0, 40.0), None);
}

#[test]
fn top_edge_resizes_unless_maximised() {
    let l = title_layout(1200.0, 1.0, &MENUS);
    assert_eq!(l.hit(Point::new(600.0, 2.0), 6.0, false), Some(TitleHit::ResizeTop));
    assert_eq!(l.hit(Point::new(4.0, 2.0), 6.0, false), Some(TitleHit::ResizeTopLeft));
    assert_eq!(l.hit(Point::new(1196.0, 2.0), 6.0, false), Some(TitleHit::ResizeTopRight));
    // Maximised: the very top pixel is the close button, so flinging the mouse to the
    // corner still hits it.
    assert_eq!(l.hit(Point::new(1196.0, 0.0), 6.0, true), Some(TitleHit::Button(CaptionButton::Close)));
}
