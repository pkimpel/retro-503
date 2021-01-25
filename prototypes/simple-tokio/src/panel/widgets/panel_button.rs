/***********************************************************************
* simple-tokio/src/widgets/button.rs
*   Module "widgets::button".
*   Lighted and unlighted panel buttons.
* Copyright (C) 2021, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2021-01-24  P.Kimpel
*   Original version, from simple-system/src/widgets/button.rs.
***********************************************************************/

use imgui::{im_str, ImStr, StyleColor, StyleVar, Ui};
use super::*;

pub struct PanelButton<'a> {
    pub position: Position,
    pub frame_size: FrameSize,
    pub off_color: Color4,
    pub on_color: Color4,
    pub active_color: Option<Color4>,
    pub border_color: Color4,
    pub border_shadow: Color4,
    pub border_size: f32,
    pub border_rounding: f32,
    pub label_color: Color4,
    pub label_text: &'a ImStr
}

impl<'a> Default for PanelButton<'a> {
    fn default() -> Self {
        let label_text = im_str!("");
        PanelButton {
            position: [0.0, 0.0],
            frame_size: [50.0, 50.0],
            off_color: GREEN_COLOR,
            on_color: GREEN_COLOR,
            active_color: None,
            border_color: GRAY_COLOR,
            border_shadow: BLACK_COLOR,
            border_size: 4.0,
            border_rounding: 1.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl<'a> PanelButton<'a> {
    pub fn build(&self, ui: &Ui, state: bool) -> bool {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        let new_color = &if state {self.on_color} else {self.off_color};
        let active_color = &match self.active_color {
            None => *new_color,
            Some(c) => c
        };
        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::BorderShadow, self.border_shadow),
            (StyleColor::Button, *new_color),
            (StyleColor::ButtonHovered, *new_color),
            (StyleColor::ButtonActive, *active_color)
        ]);

        ui.set_cursor_pos(self.position);
        let clicked = ui.button(self.label_text, self.frame_size);

        t1.pop(&ui);
        t0.pop(&ui);
        clicked
    }
}
