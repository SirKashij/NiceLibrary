//! Cover hue: the dominant hue of a Cover, from which a Detail Page takes its
//! Tint (ADR-0007). The interface turns it into colours for the shown Theme
//! and checks their contrast (see `coverTint` in src/coverTint.ts).

use serde::Serialize;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use crate::Cover;

/// The hue most of a Cover's colourful pixels share.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverHue {
    /// The hue on the colour wheel, in degrees from 0 (red) up to 360.
    pub hue: f64,
    /// How vivid the pixels of that hue are on average, from 0 (grey) to 1.
    pub saturation: f64,
}

/// The number of hue buckets on the colour wheel, 10 degrees each.
const BUCKETS: usize = 36;
/// About this many pixels are looked at; more say nothing new about the hue.
const SAMPLES: usize = 40_000;
/// A pixel this close to grey, black or white has no hue to speak of.
const MIN_SATURATION: f64 = 0.2;
const MIN_VALUE: f64 = 0.15;
/// A Cover whose colourful pixels are fewer than this share of it is treated
/// as having no colour: a speck of colour should not tint a whole page.
const MIN_COLOURFUL_SHARE: f64 = 0.03;

/// The dominant hue of `cover`; `None` when it has no colour to speak of or
/// its image cannot be read.
pub(crate) fn dominant_hue(cover: &Cover) -> Option<CoverHue> {
    let (pixels, channels) = decode(cover)?;
    let count = pixels.len() / channels;
    if count == 0 {
        return None;
    }
    let step = count.div_ceil(SAMPLES).max(1);

    // Per bucket: the pixels' weight, and the weighted sums of their hue
    // direction and saturation.
    let mut weight = [0.0_f64; BUCKETS];
    let mut along = [(0.0_f64, 0.0_f64); BUCKETS];
    let mut saturation = [0.0_f64; BUCKETS];
    let mut sampled = 0usize;
    let mut colourful = 0usize;
    for pixel in pixels.chunks_exact(channels).step_by(step) {
        sampled += 1;
        let Some((hue, sat, value)) = hsv(pixel[0], pixel[1], pixel[2]) else { continue };
        if sat < MIN_SATURATION || value < MIN_VALUE {
            continue;
        }
        colourful += 1;
        // Vivid, bright pixels say more about a Cover's colour than dull ones.
        let w = sat * value;
        let bucket = ((hue / 360.0 * BUCKETS as f64) as usize).min(BUCKETS - 1);
        let radians = hue.to_radians();
        weight[bucket] += w;
        along[bucket].0 += w * radians.cos();
        along[bucket].1 += w * radians.sin();
        saturation[bucket] += w * sat;
    }
    if (colourful as f64) < MIN_COLOURFUL_SHARE * sampled as f64 {
        return None;
    }

    // The dominant hue is the heaviest bucket together with its neighbours,
    // so a hue on a bucket's edge is not split in two.
    let around = |b: usize| [(b + BUCKETS - 1) % BUCKETS, b, (b + 1) % BUCKETS];
    let best = (0..BUCKETS).max_by(|&a, &b| {
        let sum = |b: usize| around(b).iter().map(|&i| weight[i]).sum::<f64>();
        sum(a).total_cmp(&sum(b))
    })?;
    let (mut x, mut y, mut w, mut s) = (0.0, 0.0, 0.0, 0.0);
    for i in around(best) {
        x += along[i].0;
        y += along[i].1;
        w += weight[i];
        s += saturation[i];
    }
    let hue = y.atan2(x).to_degrees().rem_euclid(360.0);
    Some(CoverHue { hue: if hue >= 360.0 { 0.0 } else { hue }, saturation: (s / w).clamp(0.0, 1.0) })
}

/// Hue in degrees, saturation and value of a pixel; `None` for pure grey.
fn hsv(r: u8, g: u8, b: u8) -> Option<(f64, f64, f64)> {
    let (r, g, b) = (f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    if delta <= 0.0 {
        return None;
    }
    let hue = if max == r {
        60.0 * ((g - b) / delta)
    } else if max == g {
        60.0 * ((b - r) / delta) + 120.0
    } else {
        60.0 * ((r - g) / delta) + 240.0
    };
    Some((hue.rem_euclid(360.0), delta / max, max))
}

/// The pixels of `cover` as 8-bit samples, with the number of channels per
/// pixel; the first three are red, green and blue.
fn decode(cover: &Cover) -> Option<(Vec<u8>, usize)> {
    match cover.media_type {
        "image/jpeg" => {
            let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
            let mut decoder = JpegDecoder::new_with_options(ZCursor::new(&cover.bytes), options);
            let pixels = decoder.decode().ok()?;
            let channels = decoder.output_colorspace()?.num_components();
            (channels >= 3).then_some((pixels, channels))
        }
        "image/png" => {
            let mut decoder = png::Decoder::new(std::io::Cursor::new(&cover.bytes));
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let mut reader = decoder.read_info().ok()?;
            let mut pixels = vec![0; reader.output_buffer_size()?];
            let frame = reader.next_frame(&mut pixels).ok()?;
            pixels.truncate(frame.buffer_size());
            let channels = frame.color_type.samples();
            (channels >= 3).then_some((pixels, channels))
        }
        _ => None,
    }
}
