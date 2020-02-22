/***********************************************************************
* panel-prototype/src/widgets/register_lamp.rs
*   Module "widgets::register_lamp".
*   Panel lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-02-16  P.Kimpel
*   Original version, cloned from widgets/lamp.rs.
***********************************************************************/

use imgui::{im_str, ImStr, StyleColor, StyleVar, Ui};
use super::*;

pub struct RegisterLamp<'a> {
    pub position: Position,
    pub frame_size: FrameSize,
    pub colors: &'a [Color4],
    pub active_color: Color4,
    pub border_color: Color4,
    pub border_shadow: Color4,
    pub border_size: f32,
    pub border_rounding: f32,
    pub label_color: Color4,
    pub label_text: &'a ImStr
}

impl<'a> Default for RegisterLamp<'a> {
    fn default() -> Self {
        let label_text = im_str!("");
        Lamp {
            position: [0.0, 0.0],
            frame_size: [12.0, 12.0],
            colors: &super::NEON_LEVEL,
            active_color: GRAY_COLOR,
            border_color: BLACK_COLOR,
            border_size: 0.0,
            border_rounding: 3.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl<'a> RegisterLamp<'a> {
    pub fn build(&self, ui: &Ui, glow: f32) -> bool{
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::Button, color),
            (StyleColor::ButtonActive, self.active_color),
            (StyleColor::ButtonHovered, color)
            ]);

        // Compute the lamp glow
        let level = (glow*(self.colors.len()-1)).round() as usize;
        let color = &self.colors[level];

        ui.set_cursor_pos(self.position);
        let clicked = ui.button(self.label_text, self.frame_size);

        t1.pop(&ui);
        t0.pop(&ui);
        clicked
    }
}
