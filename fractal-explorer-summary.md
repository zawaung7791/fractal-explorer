# Fractal explorer summary

This repository is a small web app that lets you explore fractals in your browser.
The Rust server generates image tiles, and the HTML page provides the map-like interface for panning, zooming, and changing settings.

## In plain English
- The app draws fractals such as the Mandelbrot set, Julia sets, Burning Ship, and Tricorn.
- The browser asks the server for tiny 256×256 image tiles as you move around.
- Right-clicking the Mandelbrot view jumps to a Julia set for that exact point.
- The Detail slider increases how many times the app checks each point, which makes deep zooms sharper but slower.

## How the server works
`src/main.rs` does the rendering.

1. It reads the tile request from the browser.
2. It converts each pixel into a point in the complex plane.
3. It repeatedly applies the fractal formula to see whether the point escapes.
4. It turns the escape result into a color.
5. It packages the 256×256 image as a PNG and sends it back.

## How the browser works
`static/index.html` provides the controls and navigation.

- Leaflet gives it map-style pan and zoom.
- The selected fractal, palette, density, detail, and Julia constant are sent to the server in the tile URL.
- The current view is saved in the URL hash so you can share or reload the same spot.

## How to run it
```bash
docker compose up -d --build
curl -I http://127.0.0.1:8085/
```

## Files to look at
- `src/main.rs` — fractal rendering and HTTP server
- `static/index.html` — browser UI
- `docker-compose.yml` — local container setup
- `Dockerfile` — production image build
