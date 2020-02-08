/***********************************************************************
 * panel-prototype/src/widgets/button.rs
 *  Module "widgets::button".
 *  Lighted and unlighted panel buttons.
 ***********************************************************************
 * Modification log.
 * 2020-02-07  P.Kimpel
 *     Original version.
 **********************************************************************/

use imgui::{im_str, ImStr, StyleColor, StyleVar, Ui};
use super::*;

pub struct Button<'a> {
    pub position: Position,
    pub frame_size: FrameSize,
    pub off_color: Color4,
    pub on_color: Color4,
    pub active_color: Color4,
    pub border_color: Color4,
    pub border_shadow: Color4,
    pub border_size: f32,
    pub border_rounding: f32,
    pub label_color: Color4,
    pub label_text: &'a ImStr
}

impl<'a> Default for Button<'a> {
    fn default() -> Self {
        let label_text = im_str!("");
        Button {
            position: [0.0, 0.0],
            frame_size: [50.0, 50.0],
            off_color: GREEN_COLOR, 
            on_color: GREEN_COLOR,
            active_color: GRAY_LIGHT,
            border_color: GRAY_COLOR,
            border_shadow: BLACK_COLOR,
            border_size: 6.0,
            border_rounding: 1.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl<'a> Button<'a> {
    pub fn build(&self, ui: &Ui, state: bool) -> bool {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        let new_color = &if state {self.on_color} else {self.off_color};
        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::Button, *new_color),
            (StyleColor::ButtonHovered, *new_color),
            (StyleColor::ButtonActive, self.active_color)
        ]);

        ui.set_cursor_pos(self.position);
        let clicked = ui.button(self.label_text, self.frame_size);
        
        t1.pop(&ui);
        t0.pop(&ui);
        clicked
    }
}
