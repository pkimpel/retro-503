/***********************************************************************
* panel-prototype/src/widgets/panel_lamp.rs
*   Module "widgets::panel_lamp".
*   Panel lamps.
* Copyright (C) 2020, Paul Kimpel.
* Licensed under the MIT License, see
*       http://www.opensource.org/licenses/mit-license.php
************************************************************************
* Modification log.
* 2020-02-07  P.Kimpel
*     Original version.
***********************************************************************/

use imgui::{im_str, ImStr, StyleColor, StyleVar, Ui};
use super::*;

pub struct PanelLamp<'a> {
    pub position: Position,
    pub frame_size: FrameSize,
    pub off_color: Color4,
    pub on_color: Color4,
    pub border_color: Color4,
    pub border_shadow: Color4,
    pub border_size: f32,
    pub border_rounding: f32,
    pub label_color: Color4,
    pub label_text: &'a ImStr
}

impl<'a> Default for PanelLamp<'a> {
    fn default() -> Self {
        let label_text = im_str!("");
        PanelLamp {
            position: [0.0, 0.0],
            frame_size: [50.0, 50.0],
            off_color: RED_COLOR,
            on_color: GREEN_COLOR,
            border_color: GRAY_COLOR,
            border_shadow: BLACK_COLOR,
            border_size: 4.0,
            border_rounding: 1.0,
            label_color: BLACK_COLOR,
            label_text
        }
    }
}

impl<'a> PanelLamp<'a> {
    pub fn build(&self, ui: &Ui, glow: f32) {
        let t0 = ui.push_style_vars(&[
            StyleVar::FrameRounding(self.border_rounding),
            StyleVar::FrameBorderSize(self.border_size)
        ]);

        // Compute the lamp glow
        let mut color = self.off_color.clone();
        for t in color.iter_mut().zip(self.on_color.iter()) {
            let (c, on) = t;
            *c += (*on - *c)*glow;
        }

        let t1 = ui.push_style_colors(&[
            (StyleColor::Text, self.label_color),
            (StyleColor::Border, self.border_color),
            (StyleColor::BorderShadow, self.border_shadow),
            (StyleColor::Button, color),
            (StyleColor::ButtonActive, color),
            (StyleColor::ButtonHovered, color)
        ]);

        ui.set_cursor_pos(self.position);
        let _ = ui.button(self.label_text, self.frame_size);

        t1.pop(&ui);
        t0.pop(&ui);
    }
}
