//! The widget's text: DirectWrite with the system UI font (ADR-058, Step 18 spike).
//!
//! Each run is laid out with `IDWriteTextLayout` (tabular figures, so countdowns do not
//! wobble), drawn by a software Direct2D target into a premultiplied WIC bitmap with
//! grayscale antialiasing, and composed onto the `tiny-skia` pixmap. No font is embedded;
//! the strip looks like the rest of Windows.

#![allow(unsafe_code)]

use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory,
    ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_FEATURE, DWRITE_FONT_FEATURE_TAG_TABULAR_FIGURES,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_LINE_METRICS, DWRITE_TEXT_METRICS,
    DWRITE_TEXT_RANGE, DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection,
    IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
    WICBitmapCacheOnLoad,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::{HSTRING, PCWSTR};

use super::render::{Font, Text, Weight};

/// Windows 11's UI font; Windows 10 has only Segoe UI.
const FAMILIES: [&str; 2] = ["Segoe UI Variable Text", "Segoe UI"];

pub struct DirectWrite {
    dwrite: IDWriteFactory,
    d2d: ID2D1Factory,
    wic: IWICImagingFactory,
    family: HSTRING,
    locale: HSTRING,
}

impl DirectWrite {
    /// Creates the factories on the calling thread (the widget's), which it initializes for
    /// COM if nobody did.
    pub fn new(locale: &str) -> windows::core::Result<Self> {
        // SAFETY: plain factory calls; `S_FALSE` or `RPC_E_CHANGED_MODE` from a thread that
        // already has COM are fine, WIC works in either apartment.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let family = pick_family(&dwrite);
            Ok(Self {
                d2d: D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?,
                wic: CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?,
                dwrite,
                family: HSTRING::from(family),
                locale: HSTRING::from(locale),
            })
        }
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.locale = HSTRING::from(locale);
    }

    fn layout(&self, text: &str, font: Font) -> windows::core::Result<IDWriteTextLayout> {
        let weight: DWRITE_FONT_WEIGHT = match font.weight {
            Weight::Regular => DWRITE_FONT_WEIGHT_NORMAL,
            Weight::Medium => DWRITE_FONT_WEIGHT_MEDIUM,
        };
        let utf16: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: every argument is a live COM object or a buffer that outlives the call.
        unsafe {
            let format = self.dwrite.CreateTextFormat(
                &self.family,
                None,
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font.px,
                &self.locale,
            )?;
            let layout = self
                .dwrite
                .CreateTextLayout(&utf16, &format, f32::MAX, f32::MAX)?;
            let typography = self.dwrite.CreateTypography()?;
            typography.AddFontFeature(DWRITE_FONT_FEATURE {
                nameTag: DWRITE_FONT_FEATURE_TAG_TABULAR_FIGURES,
                parameter: 1,
            })?;
            layout.SetTypography(
                &typography,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: u32::try_from(utf16.len()).unwrap_or(u32::MAX),
                },
            )?;
            Ok(layout)
        }
    }

    /// The run as a premultiplied RGBA pixmap, with its baseline `frac_x`, `frac_y` into the
    /// first pixel, and the baseline's distance from the pixmap's top.
    fn raster(
        &self,
        text: &str,
        font: Font,
        color: Color,
        frac: (f32, f32),
    ) -> windows::core::Result<Option<(Pixmap, f32)>> {
        let layout = self.layout(text, font)?;
        // SAFETY: the out structs are writable locals; the bitmap buffer is sized for the copy.
        unsafe {
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&raw mut metrics)?;
            let mut lines = [DWRITE_LINE_METRICS::default(); 1];
            let mut count = 0;
            // One line always fits; a longer layout reports an error we do not need.
            let _ = layout.GetLineMetrics(Some(&mut lines), &raw mut count);
            let baseline = lines[0].baseline;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (w, h) = (
                (metrics.widthIncludingTrailingWhitespace + frac.0 + 2.0).ceil() as u32,
                (metrics.height + frac.1 + 2.0).ceil() as u32,
            );
            if w == 0 || h == 0 {
                return Ok(None);
            }
            let bitmap = self.wic.CreateBitmap(
                w,
                h,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapCacheOnLoad,
            )?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                // Physical pixels: one DIP is one pixel, the caller scales the font.
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let target = self
                .d2d
                .CreateWicBitmapRenderTarget(&bitmap, &raw const props)?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let c = color.to_color_u8();
            let brush = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: f32::from(c.red()) / 255.0,
                    g: f32::from(c.green()) / 255.0,
                    b: f32::from(c.blue()) / 255.0,
                    a: f32::from(c.alpha()) / 255.0,
                },
                None,
            )?;
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            target.DrawTextLayout(
                windows_numerics::Vector2 {
                    X: frac.0,
                    Y: frac.1,
                },
                &layout,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            target.EndDraw(None, None)?;
            let mut buf = vec![0u8; (w * h * 4) as usize];
            bitmap.CopyPixels(std::ptr::null(), w * 4, &mut buf)?;
            for p in buf.as_chunks_mut::<4>().0 {
                p.swap(0, 2);
            }
            let size = IntSize::from_wh(w, h).expect("non-zero");
            Ok(Pixmap::from_vec(buf, size).map(|p| (p, frac.1 + baseline)))
        }
    }
}

/// The first of [`FAMILIES`] this system has.
fn pick_family(dwrite: &IDWriteFactory) -> &'static str {
    // SAFETY: out parameters are writable locals.
    unsafe {
        let mut collection: Option<IDWriteFontCollection> = None;
        if dwrite
            .GetSystemFontCollection(&raw mut collection, false)
            .is_err()
        {
            return FAMILIES[1];
        }
        let Some(collection) = collection else {
            return FAMILIES[1];
        };
        for family in FAMILIES {
            let name = HSTRING::from(family);
            let (mut index, mut exists) = (0, false.into());
            if collection
                .FindFamilyName(PCWSTR(name.as_ptr()), &raw mut index, &raw mut exists)
                .is_ok()
                && exists.as_bool()
            {
                return family;
            }
        }
        FAMILIES[1]
    }
}

impl Text for DirectWrite {
    fn width(&mut self, text: &str, font: Font) -> f32 {
        let Ok(layout) = self.layout(text, font) else {
            return 0.0;
        };
        let mut metrics = DWRITE_TEXT_METRICS::default();
        // SAFETY: a writable local out struct.
        match unsafe { layout.GetMetrics(&raw mut metrics) } {
            Ok(()) => metrics.widthIncludingTrailingWhitespace,
            Err(_) => 0.0,
        }
    }

    fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        font: Font,
        x: f32,
        baseline: f32,
        color: Color,
    ) {
        // The run is drawn at its sub-pixel offset and placed on whole pixels.
        let (left, frac_x) = (x.floor(), x - x.floor());
        match self.raster(text, font, color, (frac_x, 0.0)) {
            Ok(Some((run, run_baseline))) => {
                let top = baseline - run_baseline;
                let (top, frac_y) = (top.floor(), top - top.floor());
                // Re-raster with the vertical fraction only when it matters (rarely: the
                // caller rounds baselines to whole pixels).
                let run = if frac_y > 0.01 {
                    match self.raster(text, font, color, (frac_x, frac_y)) {
                        Ok(Some((run, _))) => run,
                        _ => run,
                    }
                } else {
                    run
                };
                #[allow(clippy::cast_possible_truncation)]
                pixmap.draw_pixmap(
                    left as i32,
                    top as i32,
                    run.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
            Ok(None) => {}
            Err(error) => tracing::debug!(%error, "widget: text not drawn"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: Font = Font {
        px: 12.0,
        weight: Weight::Regular,
    };

    #[test]
    fn directwrite_measures_and_draws_turkish_text() {
        let mut dw = DirectWrite::new("tr-TR").expect("factories");
        let w = dw.width("7g %34 · Çar İ", FONT);
        assert!(w > 40.0 && w < 160.0, "{w}");
        assert!(dw.width("Çar", FONT) > dw.width("Ç", FONT));
        let mut p = Pixmap::new(160, 24).expect("pixmap");
        dw.draw(&mut p, "Çar İ", FONT, 4.3, 16.0, Color::WHITE);
        let inked = p.pixels().iter().filter(|c| c.alpha() > 128).count();
        assert!(inked > 20, "{inked}");
        // Nothing above the dotted İ's dot or far below the baseline.
        let below = (21..24)
            .flat_map(|y| (0..160).map(move |x| (x, y)))
            .filter(|&(x, y)| p.pixel(x, y).is_some_and(|c| c.alpha() > 0))
            .count();
        assert_eq!(below, 0);
    }

    #[test]
    fn directwrite_figures_are_tabular() {
        let mut dw = DirectWrite::new("en-US").expect("factories");
        let font = Font {
            px: 12.0,
            weight: Weight::Medium,
        };
        assert!((dw.width("11:11", font) - dw.width("00:00", font)).abs() < 0.01);
    }
}
