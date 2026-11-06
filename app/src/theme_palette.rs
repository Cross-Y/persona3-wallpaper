//! Pure color extraction and palette transformations, independent of desktop APIs.
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Palette {
    pub accent: String, pub secondary: String, pub tertiary: String, pub quaternary: String,
    pub background: String, pub surface: String, pub on_accent: String,
}
impl Palette {
    pub(crate) fn json(&self) -> Value {
        json!({"primary":self.accent,"secondary":self.secondary,"tertiary":self.tertiary,
            "quaternary":self.quaternary,"background":self.background,"surface":self.surface,
            "on_accent":self.on_accent})
    }
    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        Some(Self { accent:valid_hex(value["primary"].as_str()?)?,
            secondary:valid_hex(value["secondary"].as_str()?)?,
            tertiary:valid_hex(value["tertiary"].as_str()?)?,
            quaternary:valid_hex(value["quaternary"].as_str()?)?,
            background:valid_hex(value["background"].as_str()?)?,
            surface:valid_hex(value["surface"].as_str()?)?,
            on_accent:valid_hex(value["on_accent"].as_str()?)? })
    }
}
pub(crate) fn valid_hex(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    if hex.len()!=6 || !hex.bytes().all(|b|b.is_ascii_hexdigit()) { return None; }
    Some(format!("#{}",hex.to_ascii_uppercase()))
}
fn rgb_value(hex: &str) -> [u8;3] {
    let hex = hex.trim_start_matches('#');
    [0,2,4].map(|start| u8::from_str_radix(&hex[start..start+2],16).unwrap_or(0))
}
pub(crate) fn text_on(hex: &str) -> String {
    let c=rgb_value(hex);
    let linear=|v:u8| { let x=v as f64/255.0; if x<=0.04045 { x/12.92 } else { ((x+0.055)/1.055).powf(2.4) } };
    let luminance=0.2126*linear(c[0])+0.7152*linear(c[1])+0.0722*linear(c[2]);
    if luminance > 0.179 { "#13202E".into() } else { "#FFFFFF".into() }
}
pub(crate) fn palette_with_overrides(mut p: Palette, settings: &Value) -> Palette {
    for role in ["primary","secondary","tertiary","quaternary"] {
        if let Some(color)=settings["theme_manual_colors"][role].as_str().and_then(valid_hex) {
            match role { "primary"=>p.accent=color, "secondary"=>p.secondary=color,
                "tertiary"=>p.tertiary=color, _=>p.quaternary=color }
        }
    }
    p.on_accent=text_on(&p.accent);
    p
}
pub(crate) fn fallback_palette(settings: &Value) -> Palette {
    let accent=settings["theme_last_accent"].as_str().and_then(valid_hex).unwrap_or_else(||"#559DD7".into());
    let c=rgb_value(&accent);
    Palette { accent:accent.clone(), secondary:rgb_hex(mix(c,[190,205,225],0.38)),
        tertiary:rgb_hex(mix(c,[c[2],c[0],c[1]],0.38)),
        quaternary:rgb_hex(mix(c,[255,188,138],0.42)),
        background:rgb_hex(mix([13,18,27],c,0.10)),surface:rgb_hex(mix([27,34,47],c,0.16)),
        on_accent:text_on(&accent) }
}

fn rgb_hex(rgb: [u8;3]) -> String { format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]) }
fn mix(a: [u8;3], b: [u8;3], amount: f64) -> [u8;3] {
    std::array::from_fn(|i| (a[i] as f64 * (1.0 - amount) + b[i] as f64 * amount).round().clamp(0.0, 255.0) as u8)
}
fn rgb_luminance(rgb: [u8;3]) -> f64 { 0.2126 * rgb[0] as f64 + 0.7152 * rgb[1] as f64 + 0.0722 * rgb[2] as f64 }

/// Compact, deterministic 12-sector histogram. Saturated mid-bright colors
/// score higher than tiny highlights, dark backgrounds, or white text.
pub fn palette_from_rgb(pixels: &[u8]) -> Result<Palette, String> {
    if pixels.len() < 3 || pixels.len() % 3 != 0 { return Err("Invalid RGB image sample".into()); }
    let mut bins = [(0f64, [0f64;3]); 12];
    let mut average = [0f64;3];
    for pixel in pixels.chunks_exact(3) {
        let c = [pixel[0],pixel[1],pixel[2]];
        for i in 0..3 { average[i] += c[i] as f64; }
        let max = *c.iter().max().unwrap_or(&0) as f64;
        let min = *c.iter().min().unwrap_or(&0) as f64;
        let range = max - min;
        if max < 45.0 || range < 20.0 { continue; }
        let saturation = range / max;
        let luminance = rgb_luminance(c);
        if luminance < 32.0 || luminance > 248.0 { continue; }
        let hue = if max == c[0] as f64 { (c[1] as f64-c[2] as f64)/range }
            else if max == c[1] as f64 { 2.0 + (c[2] as f64-c[0] as f64)/range }
            else { 4.0 + (c[0] as f64-c[1] as f64)/range };
        let sector = ((hue * 2.0).rem_euclid(12.0).floor() as usize).min(11);
        let weight = 0.35 + saturation * 1.7;
        bins[sector].0 += weight;
        for i in 0..3 { bins[sector].1[i] += c[i] as f64 * weight; }
    }
    let total = (pixels.len()/3) as f64;
    let fallback = std::array::from_fn(|i| (average[i]/total).round().clamp(0.0,255.0) as u8);
    let (weight, sums) = bins.iter().max_by(|a,b| a.0.total_cmp(&b.0))
        .expect("The histogram has twelve sectors");
    let color = if *weight > 0.0 {
        std::array::from_fn(|i| (sums[i] / *weight).round().clamp(0.0,255.0) as u8)
    } else { fallback };
    let secondary = mix(color, [190,205,225], 0.38);
    let tertiary = mix(color, [color[2],color[0],color[1]], 0.38);
    let quaternary = mix(color, [255,188,138], 0.42);
    let background = mix([13,18,27],color,0.10);
    let surface = mix([27,34,47],color,0.16);
    let accent=rgb_hex(color);
    Ok(Palette { on_accent:text_on(&accent), accent, secondary:rgb_hex(secondary),
        tertiary:rgb_hex(tertiary),quaternary:rgb_hex(quaternary),
        background:rgb_hex(background),surface:rgb_hex(surface) })
}

