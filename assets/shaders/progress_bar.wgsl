// Progress bar drawn into the trough of an appliance popup.
//
// A pill that grows from the left end of the trough to its full length as
// `progress` goes from 0 to 1, with a dark outline and a lighter highlight band
// in its upper half. Geometry is in popup sprite pixels; the quad is QUAD_SIZE
// pixels and centred on the trough.

struct ProgressBarMaterial {
  color: vec4<f32>,
  highlight_color: vec4<f32>,
  outline_color: vec4<f32>,
  // Fraction of the bar filled, 0..=1.
  progress: f32,
}

@group(2) @binding(0) var<uniform> material: ProgressBarMaterial;

// Must match PROGRESS_QUAD in appliance_popup.rs; leaves a pixel around the
// bar for anti-aliasing.
const QUAD_SIZE: vec2<f32> = vec2<f32>(76.0, 14.0);
// Size of the full bar.
const BAR_SIZE: vec2<f32> = vec2<f32>(74.0, 12.0);
const RADIUS: f32 = 6.0;
const OUTLINE: f32 = 1.5;
// The highlight band, as heights above the bar's centre line, and how far it
// stays clear of the outline at the round ends.
const HIGHLIGHT_BOTTOM: f32 = 0.6;
const HIGHLIGHT_TOP: f32 = 3.0;
const HIGHLIGHT_INSET: f32 = 2.5;

struct FragmentInput {
  @location(2) uv: vec2<f32>,
}

// Signed distance to a horizontal pill centred at `center_x` (negative inside).
fn pill_distance(p: vec2<f32>, center_x: f32, half_length: f32) -> f32 {
  let straight = max(half_length - RADIUS, 0.0);
  let q = vec2<f32>(max(abs(p.x - center_x) - straight, 0.0), p.y);
  return length(q) - RADIUS;
}

// Anti-aliased coverage of the region where `d` is negative.
fn coverage(d: f32, aa: f32) -> f32 {
  return clamp(0.5 - d / aa, 0.0, 1.0);
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
  if material.progress <= 0.0 {
    discard;
  }
  // uv runs top-down; flip y so +y is up.
  let p = (vec2<f32>(in.uv.x, 1.0 - in.uv.y) - 0.5) * QUAD_SIZE;

  // Even an almost empty bar is a full circle, so it never looks clipped.
  let left = -BAR_SIZE.x / 2.0;
  let bar_length = mix(BAR_SIZE.y, BAR_SIZE.x, clamp(material.progress, 0.0, 1.0));
  let d = pill_distance(p, left + bar_length / 2.0, bar_length / 2.0);

  let aa = max(fwidth(d), 0.0001);
  let shape = coverage(d, aa);
  if shape <= 0.0 {
    discard;
  }

  let band = min(coverage(HIGHLIGHT_BOTTOM - p.y, aa), coverage(p.y - HIGHLIGHT_TOP, aa));
  let highlight = band * coverage(d + HIGHLIGHT_INSET, aa);
  let fill = mix(material.color, material.highlight_color, highlight);

  // Outline in the outermost OUTLINE pixels, fill inside.
  let inside = coverage(d + OUTLINE, aa);
  let color = mix(material.outline_color, fill, inside);
  return vec4<f32>(color.rgb, color.a * shape);
}
