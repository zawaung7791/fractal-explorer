//! Fractal Explorer: a tiny Rust tile server.
//!
//! The browser (Leaflet) asks for 256x256 tiles at /tile/{z}/{x}/{y}.
//! At zoom 0 one tile covers a 4x4 square of the complex plane; every
//! zoom level halves that, exactly like a slippy map.

use axum::{
    extract::{Path, Query},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::f64::consts::{LN_2, TAU};

const TILE: usize = 256;
const MAX_ZOOM: u32 = 46; // f64 runs out of precision around here
const INDEX: &str = include_str!("../static/index.html");

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Mandelbrot,
    Julia,
    BurningShip,
    Tricorn,
}

impl Kind {
    fn parse(s: &str) -> Kind {
        match s {
            "julia" => Kind::Julia,
            "ship" => Kind::BurningShip,
            "tricorn" => Kind::Tricorn,
            _ => Kind::Mandelbrot,
        }
    }

    /// Centre of the zoom-0 view. Must match CENTERS in index.html.
    fn center(self) -> (f64, f64) {
        match self {
            Kind::Mandelbrot => (-0.5, 0.0),
            Kind::Julia => (0.0, 0.0),
            Kind::BurningShip => (-0.4, 0.5),
            Kind::Tricorn => (-0.3, 0.0),
        }
    }
}

fn default_kind() -> String {
    "mandelbrot".into()
}
fn one() -> f64 {
    1.0
}
fn default_cre() -> f64 {
    -0.8
}
fn default_cim() -> f64 {
    0.156
}

#[derive(Deserialize)]
struct Params {
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    palette: u8,
    /// Colour bands per escape-time unit (higher = more stripes)
    #[serde(default = "one")]
    density: f64,
    /// Multiplier on the automatic iteration count
    #[serde(default = "one")]
    detail: f64,
    /// Julia constant
    #[serde(default = "default_cre")]
    cre: f64,
    #[serde(default = "default_cim")]
    cim: f64,
}

/// Smooth escape time, or None if the point never escaped (inside the set).
#[inline(always)]
fn escape(kind: Kind, px: f64, py: f64, jre: f64, jim: f64, max_iter: u32) -> Option<f64> {
    let (mut x, mut y, cr, ci) = match kind {
        Kind::Julia => (px, py, jre, jim),
        // Flip the ship so it sails upright
        Kind::BurningShip => (0.0, 0.0, px, -py),
        _ => (0.0, 0.0, px, py),
    };

    // Skip the two biggest Mandelbrot regions: they never escape
    if kind == Kind::Mandelbrot {
        let xq = px - 0.25;
        let q = xq * xq + py * py;
        if q * (q + xq) <= 0.25 * py * py {
            return None;
        }
        if (px + 1.0) * (px + 1.0) + py * py <= 0.0625 {
            return None;
        }
    }

    for n in 0..max_iter {
        let x2 = x * x;
        let y2 = y * y;
        if x2 + y2 > 256.0 {
            // Normalised iteration count for smooth colour gradients
            let log_zn = (x2 + y2).ln() * 0.5;
            let nu = (log_zn / LN_2).ln() / LN_2;
            return Some(n as f64 + 1.0 - nu);
        }
        let xy = x * y;
        match kind {
            Kind::BurningShip => {
                y = 2.0 * xy.abs() + ci;
            }
            Kind::Tricorn => {
                y = -2.0 * xy + ci;
            }
            _ => {
                y = 2.0 * xy + ci;
            }
        }
        x = x2 - y2 + cr;
    }
    None
}

/// Cosine palettes: colour = a + b * cos(2π(c·t + d))
fn color(v: Option<f64>, palette: u8, density: f64) -> [u8; 3] {
    let Some(v) = v else { return [0, 0, 0] };
    let t = v.max(0.0).sqrt() * 0.18 * density;
    let (a, b, c, d) = match palette {
        1 => ([0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [1.0, 1.0, 0.5], [0.80, 0.90, 0.30]), // ember
        2 => ([0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [1.0, 1.0, 1.0], [0.30, 0.20, 0.20]), // lagoon
        3 => ([0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [2.0, 1.0, 0.0], [0.50, 0.20, 0.25]), // electric
        4 => ([0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]),    // silver
        _ => ([0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [1.0, 1.0, 1.0], [0.0, 0.10, 0.20]),  // rainbow
    };
    let mut out = [0u8; 3];
    for i in 0..3 {
        let ch = a[i] + b[i] * (TAU * (c[i] * t + d[i])).cos();
        out[i] = (ch.clamp(0.0, 1.0) * 255.0) as u8;
    }
    out
}

fn render_tile(z: u32, tx: i64, ty: i64, p: &Params) -> Vec<u8> {
    let kind = Kind::parse(&p.kind);
    let (cx, cy) = kind.center();
    let span = 4.0 / (1u64 << z) as f64; // width of one tile in the complex plane
    let step = span / TILE as f64;
    let re0 = (cx - 2.0) + tx as f64 * span;
    let im0 = (cy + 2.0) - ty as f64 * span; // y grows downwards on screen

    let detail = p.detail.clamp(0.25, 8.0);
    let max_iter = ((150.0 + 45.0 * z as f64) * detail).min(30_000.0) as u32;
    let density = p.density.clamp(0.1, 10.0);

    let mut buf = vec![0u8; TILE * TILE * 3];
    for py in 0..TILE {
        let ci = im0 - (py as f64 + 0.5) * step;
        for px in 0..TILE {
            let cr = re0 + (px as f64 + 0.5) * step;
            let v = escape(kind, cr, ci, p.cre, p.cim, max_iter);
            let rgb = color(v, p.palette, density);
            let i = (py * TILE + px) * 3;
            buf[i..i + 3].copy_from_slice(&rgb);
        }
    }
    buf
}

fn encode_png(rgb: &[u8]) -> Result<Vec<u8>, png::EncodingError> {
    let mut out = Vec::with_capacity(64 * 1024);
    {
        let mut enc = png::Encoder::new(&mut out, TILE as u32, TILE as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header()?;
        writer.write_image_data(rgb)?;
    }
    Ok(out)
}

async fn tile(Path((z, x, y)): Path<(u32, i64, i64)>, Query(p): Query<Params>) -> Response {
    if z > MAX_ZOOM {
        return (StatusCode::BAD_REQUEST, "Zoom level too deep").into_response();
    }
    // Rendering is CPU-bound, so keep it off the async worker threads.
    // Tokio's blocking pool renders many tiles in parallel across your cores.
    let result = tokio::task::spawn_blocking(move || encode_png(&render_tile(z, x, y, &p))).await;

    match result {
        Ok(Ok(bytes)) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "public, max-age=604800, immutable"),
            ],
            bytes,
        )
            .into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Tile render failed").into_response(),
    }
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let app = Router::new()
        .route("/", get(|| async { Html(INDEX) }))
        .route("/tile/:z/:x/:y", get(tile));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("could not bind port");
    println!("Fractal explorer listening on port {port}");
    axum::serve(listener, app).await.expect("server error");
}
