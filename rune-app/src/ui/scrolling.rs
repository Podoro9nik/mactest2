//! Persistent, visible scroll affordances, driven by the viewport's live geometry.
//! The native wheel keeps its platform behavior; holding the middle button pans the
//! content directly. No timer or application-state update is needed while scrolling.
use super::{State, color};
use rune::{Cursor, El, Live, Phase, PointerButton, ScrollPos, el};
use std::{cell::Cell, rc::Rc};

const RAIL: f32 = 16.0;
const INSET: f32 = 2.0;
const MIN_THUMB: f32 = 32.0;

#[derive(Clone, Copy, Default)]
struct Geometry {
    offset: f32,
    max: f32,
    track: f32,
    thumb: f32,
    top: f32,
}
impl Geometry {
    fn from_position(at: ScrollPos) -> Self {
        let max = at.max[1].max(0.0);
        let track = (at.drawn[1] - 2.0 * INSET).max(0.0);
        let content = at.viewport[1] + max;
        let thumb = if content > 0.0 {
            (track * at.viewport[1] / content).max(MIN_THUMB).min(track)
        } else {
            track
        };
        let offset = at.offset[1].clamp(0.0, max);
        let top = if max > 0.0 {
            offset / max * (track - thumb)
        } else {
            0.0
        };
        Self {
            offset,
            max,
            track,
            thumb,
            top,
        }
    }
    fn at_thumb(self, top: f32) -> f32 {
        let travel = self.track - self.thumb;
        if travel > 0.0 {
            (top / travel * self.max).clamp(0.0, self.max)
        } else {
            0.0
        }
    }
}

pub(super) trait ScrollControls {
    fn scroll_controls(self) -> El;
}
impl ScrollControls for El {
    fn scroll_controls(self) -> El {
        // Preserve the viewport's flex placement while putting the rail outside
        // its scrolling content. Keys remain stable for reveal/page commands.
        let (area, key, outer, viewport) = self.wrap_scroll_area((file!(), line!(), column!()));
        let target = format!("{area}/{key}");
        let geometry = Rc::new(Cell::new(Geometry::default()));
        let track = Live::new([0.0; 4]);
        let thumb = Live::new([0.0; 4]);
        let (watch, drag, pan, wheel) = (
            geometry.clone(),
            geometry.clone(),
            geometry.clone(),
            geometry,
        );
        let (track_position, thumb_position) = (track.clone(), thumb.clone());
        let (pan_target, wheel_target) = (target.clone(), target.clone());
        outer
            .child(
                viewport
                    .pr(RAIL + 6.0)
                    .pointer_buttons([PointerButton::Middle])
                    .on_drag_any(move |d, tx| {
                        tx.live_only();
                        if tx.button() != PointerButton::Middle {
                            return;
                        }
                        let at = pan.get();
                        match d.phase {
                            Phase::Down => d.data = at.offset,
                            Phase::Move => tx.scroll_to_instant(
                                &pan_target,
                                0.0,
                                (d.data - d.dy).clamp(0.0, at.max),
                            ),
                            Phase::Up => (),
                        }
                    })
                    .chain_scroll_offset(move |at| {
                        let g = Geometry::from_position(at);
                        watch.set(g);
                        if g.max <= 0.5 || g.track <= 0.0 {
                            track_position.set([0.0; 4]);
                            thumb_position.set([0.0; 4]);
                        } else {
                            track_position.set([
                                (at.drawn[0] - RAIL).max(0.0),
                                INSET,
                                RAIL,
                                g.track,
                            ]);
                            thumb_position.set([3.0, g.top, RAIL - 6.0, g.thumb]);
                        }
                    }),
            )
            .child(
                el().key(format!("{key}-track"))
                    .absolute(0.0, 0.0)
                    .live_rect(&track)
                    .fill(color("#F2E7E0"))
                    .radius(RAIL / 2.0)
                    .a11y_hidden()
                    .on_wheel(move |_: &mut State, event, tx| {
                        tx.live_only();
                        let at = wheel.get();
                        if at.max <= 0.0 {
                            return false;
                        }
                        tx.scroll_to_instant(
                            &wheel_target,
                            0.0,
                            (at.offset - event.dy).clamp(0.0, at.max),
                        );
                        true
                    })
                    .on_drag_any(move |d, tx| {
                        tx.live_only();
                        let at = drag.get();
                        if at.max <= 0.0 || d.cancelled() {
                            return;
                        }
                        match d.phase {
                            Phase::Down => {
                                // Keep the grab point when grabbing the thumb;
                                // clicking an empty part of the track jumps there.
                                d.data = if (at.top..=at.top + at.thumb).contains(&d.y) {
                                    d.y - at.top
                                } else {
                                    at.thumb / 2.0
                                };
                                tx.scroll_to_instant(&target, 0.0, at.at_thumb(d.y - d.data));
                            }
                            Phase::Move => {
                                tx.scroll_to_instant(&target, 0.0, at.at_thumb(d.y - d.data))
                            }
                            Phase::Up => (),
                        }
                    })
                    .child(
                        el().key(format!("{key}-thumb"))
                            .absolute(0.0, 0.0)
                            .live_rect(&thumb)
                            .fill(color("#C49AA6"))
                            .hover(color("#AE7D8D"))
                            .radius(5.0)
                            .cursor(Cursor::Grab),
                    ),
            )
    }
}
