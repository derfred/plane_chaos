// Countdown ring drawn around a thought bubble.
//
// The ring starts at 12 o'clock and runs clockwise over `progress` of the full
// circle, with round caps and a dark outline. Geometry is in bubble sprite
// pixels; the quad is RING_BOX pixels wide and centred on the bubble circle.

struct TimerRingMaterial {
  color: vec4<f32>,
  outline_color: vec4<f32>,
  // Fraction of the ring still shown, 0..=1.
  progress: f32,
}

@group(2) @binding(0) var<uniform> material: TimerRingMaterial;

const RING_BOX: f32 = 100.0;
// Radius of the centre line of the ring.
const RADIUS: f32 = 43.0;
// Half the ring width, including the outline.
const HALF_WIDTH: f32 = 5.5;
const OUTLINE: f32 = 1.5;
const TAU: f32 = 6.28318530718;

struct FragmentInput {
  @location(2) uv: vec2<f32>,
}

// Signed distance to the ring arc (negative inside).
fn arc_distance(p: vec2<f32>, sweep: f32) -> f32 {
  let ring = abs(length(p) - RADIUS) - HALF_WIDTH;
  if sweep >= TAU {
    return ring;
  }
  // Angle clockwise from 12 o'clock, in 0..TAU.
  var angle = atan2(p.x, p.y);
  if angle < 0.0 {
    angle += TAU;
  }
  if angle <= sweep {
    return ring;
  }
  // Outside the arc: distance to the nearer round cap.
  let start_cap = vec2<f32>(0.0, RADIUS);
  let end_cap = RADIUS * vec2<f32>(sin(sweep), cos(sweep));
  return min(length(p - start_cap), length(p - end_cap)) - HALF_WIDTH;
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
  if material.progress <= 0.0 {
    discard;
  }
  // uv runs top-down; flip y so +y is up.
  let p = (vec2<f32>(in.uv.x, 1.0 - in.uv.y) - 0.5) * RING_BOX;
  let d = arc_distance(p, material.progress * TAU);

  let aa = max(fwidth(d), 0.0001);
  let coverage = clamp(0.5 - d / aa, 0.0, 1.0);
  if coverage <= 0.0 {
    discard;
  }
  // Outline in the outermost OUTLINE pixels, fill inside.
  let fill = clamp(0.5 - (d + OUTLINE) / aa, 0.0, 1.0);
  let color = mix(material.outline_color, material.color, fill);
  return vec4<f32>(color.rgb, color.a * coverage);
}
