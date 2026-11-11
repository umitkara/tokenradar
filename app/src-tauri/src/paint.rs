//! Color helpers shared by the natively drawn surfaces (tray icon, widget).

/// sRGB bytes of an OKLCH color, to check hard-coded colors against the design tokens.
#[cfg(test)]
#[allow(clippy::many_single_char_names)] // the names of the OKLab formulas
pub fn oklch_to_srgb8(l: f32, c: f32, h: f32) -> [u8; 3] {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.396_337_8 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_35 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let lin = [
        4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
        -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
    ];
    lin.map(|x| {
        let x = x.clamp(0.0, 1.0);
        let e = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (e * 255.0).round() as u8;
        v
    })
}
