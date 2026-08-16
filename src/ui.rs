//! UI rendering utilities for buttons, panels, and progress bars

mod bounds;
mod contrast;
mod font;
mod format;
mod forms;
mod hover_tooltip;
mod layout;
mod number;
mod plaque;
mod pointer;
mod pseudo;
mod scroll_tabs;
mod surfaces;
mod widgets;

pub use bounds::{
    auditing, begin_audit, begin_collision_audit, current as current_region, current_surface,
    note_control, take_audit, Decorative, Finding, Region,
};
pub use contrast::{
    darken_until, flatten, passes as contrast_passes, ratio as contrast_ratio, Level,
};
pub use font::*;
pub use format::*;
pub use forms::*;
pub use hover_tooltip::*;
pub use layout::*;
pub use number::*;
pub use plaque::*;
pub use pointer::{
    begin_target_audit, begin_target_frame, end_frame_neighbours, end_target_audit,
    neighbours_warm, note_neighbour, note_target, overlapping_targets, smallest_touchable_width,
    touch_area, touch_area_among, touch_area_among_for_scale, touch_area_for_scale,
    undersized_targets, Pointer, MIN_TARGET,
};
pub use pseudo::{
    active as pseudo_active, disable as pseudo_disable, enable as pseudo_enable,
    Once as PseudoOnce, Pseudo,
};
pub use scroll_tabs::*;
pub use surfaces::*;
pub use widgets::*;
