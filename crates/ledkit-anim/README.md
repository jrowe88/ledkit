# ledkit-anim

Animations for LED strips and matrices: pick an effect by the name you know
(rainbow, comet, sparkle, flames, swirl, rings ...), change a few settings, and
run it. Combine effects without writing new ones: one after another with fades
and wipes, stacked in layers, or different effects on different parts of a strip.
`no_std`, no allocator.

Part of [ledkit](https://github.com/jrowe88/ledkit).

## A first animation

An animation draws "what the LEDs look like at this time". Give it the time
since the program started, then `show` the strip as usual:

```rust
use ledkit_anim::prelude::*;
use ledkit_core::prelude::*;
use ledkit_core::testing::MemoryOutput;

// On a real board, the output comes from a backend such as ledkit-esp or ledkit-spi.
let mut strip = Strip::<Ws2812b, 30, _>::new(MemoryOutput::<_, 30>::new());
strip.set_brightness(Brightness::from_percent(20));

let comet = Comet::new().color(Rgb::RED).tail_length(6);

for millis in (0..1000).step_by(20) {
    // In your program: the time since start, from your board's clock.
    let now = Time::from_millis(millis);
    comet.render(&mut strip, now);
    strip.show().unwrap();
}
```

Because effects are drawn from the time, not by counting frames, they run at the
same speed however fast your loop goes.

**The time.** `Time` is made from a `core::time::Duration` (what
`std::time::Instant::elapsed` gives) or from milliseconds. Board clocks usually have
their own duration type; with esp-hal, for example:

```rust,ignore
let start = esp_hal::time::Instant::now();
// ... each frame:
let now = Time::from_millis(start.elapsed().as_millis() as u32);
```

**On a matrix**, render onto a `Canvas` from `ledkit-layout`, which knows how the
panel is wired. Every effect works on strips and matrices:

```rust
use ledkit_anim::prelude::*;
use ledkit_core::prelude::*;
use ledkit_core::testing::MemoryOutput;
use ledkit_layout::{Canvas, MatrixLayout};

const LAYOUT: MatrixLayout = MatrixLayout::new(8, 8).serpentine();
let mut strip = Strip::<Ws2812b, 64, _>::new(MemoryOutput::<_, 64>::new());
let swirl = Swirl::new().tightness(48);

swirl.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), Time::from_millis(0));
strip.show().unwrap();
```

`render` only sets the colors. Brightness, gamma, dithering and the power limit
are applied by `show`, as always. You can also render onto part of a strip (a
`SegmentMut` from `ledkit-layout`) or a plain array of colors.

## The effects

Every effect works with no settings at all (`Rainbow::new()`), and has setters with
plain names for the rest. Times are `core::time::Duration`s; speeds are in LEDs per
second.

| Effect | Also known as | Settings |
|---|---|---|
| `Solid` | static | `color` |
| `Blink` | strobe, flash | `color`, `off_color`, `period`, `duty` |
| `Breathe` | pulse | `color`, `period`, `levels(min, max)` |
| `Rainbow` | color loop | `cycle_time`, `width`, `saturation`, `direction` |
| `ColorCycle` | fade | colors, `cycle_time` |
| `GradientScroll` | palette scroll | colors, `cycle_time`, `width`, `direction` |
| `ColorWipe` | wipe, fill, loading | `color`, `background`, `wipe_time`, `direction`, `looping` |
| `Chase` | theater chase, marquee, running lights | `color`, `background`, `size`, `spacing`, `speed`, `direction` |
| `Comet` | meteor, scanner, Larson, Cylon | `color`, `tail_length`, `speed`, `bounce`, `direction` |
| `Sparkle` | twinkle, glitter | colors, `background`, `density`, `twinkle_time` |
| `Flames` | fire, candle | colors, `size`, `speed` |
| `Swirl` | spiral | colors, `tightness`, `arms`, `turn_time`, `direction`, `center` |
| `Rings` | concentric | colors, `ring_width`, `cycle_time`, `outward`, `center` |
| `Pinwheel` | rotating arms, rotating X | `color`, `background`, `arms`, `turn_time`, `direction`, `center` |
| `Plasma` | | colors, `scale`, `cycle_time` |
| `Rain` | matrix rain, digital rain | `color`, `density`, `speed`, `trail_length` |

The first eleven run along a strip (on a matrix, along each row; `Chase` follows the
wiring). `Flames` rises from the bottom row of a matrix. The last five are made for
matrices and still do something sensible on a strip.

"Colors" means four setters: `.color(c)` for one color, `.colors_between(a, b)` for a
blend, `.palette(&palettes::FIRE)` for a color map, or `.colors(Colors::...)`.

```rust
use core::time::Duration;
use ledkit_anim::prelude::*;

let candle = Flames::new().colors_between(Rgb::new(120, 20, 0), Rgb::GOLD).size(60);
let ocean = Rings::new().palette(&palettes::OCEAN).outward(false);
let scanner = Comet::new().color(Rgb::RED).bounce(true).speed(20);
let alarm = Blink::new().color(Rgb::RED).period(Duration::from_millis(300));
```

**Palettes.** `ledkit_color::palettes` (also in this crate's prelude) has
`RAINBOW`, `RED_ORANGE`, `JET`, `FIRE`, `HEAT`, `LAVA`, `OCEAN`, `FOREST`,
`CLOUD`, `PARTY`, `SUNSET`, `ICE` and `VIRIDIS`. Make your own with
`ledkit_color::Gradient::new` or `Palette::new`, as a `const` or `static`.

**By name.** `Effect` holds any built-in effect, so you can choose one at run time
from a name, with the other names above working too:

```rust
use ledkit_anim::prelude::*;

let effect = Effect::from_name("theater chase").unwrap();
assert_eq!(effect.name(), "chase");
for effect in Effect::ALL {
    // every effect, with its default settings
}
```

## Combining effects

The `AnimationExt` methods (in the prelude) work on any animation:

```rust
use core::time::Duration;
use ledkit_anim::prelude::*;

// A white comet over a dim rainbow. `Blend::Over` lets black on top show
// what's underneath, and the comet's fading tail blends into it.
let comet_over_rainbow = Rainbow::new()
    .dimmed(60)
    .layer(Comet::new().color(Rgb::WHITE), Blend::Over);

// Sparkle for 10 seconds, then crossfade into flames over 2 seconds.
let evening = Sparkle::new()
    .crossfade_to(Flames::new(), Duration::from_secs(10), Duration::from_secs(2));

// A rainbow running the other way, half as fast, mirrored from the middle.
let mirrored = Rainbow::new().reversed().at_speed(50).mirrored();
```

- `reversed`, `mirrored`, `at_speed(percent)`, `starting_at(time)`, `dimmed(level)`
- `layer(top, blend)`, with blends `Over`, `Add`, `Multiply`, `Screen`, `Lighten`,
  `Darken` and `Average`
- `then(next, at, transition)` and `crossfade_to(next, at, over)`

**Playlists** play animations one after another, each for its own time, looping:

```rust
use core::time::Duration;
use ledkit_anim::prelude::*;

let swirl = Swirl::new();
let rings = Rings::new();
let flames = Flames::new();
let second = Duration::from_secs(1);

let entries = [
    Entry::new(&swirl, 8 * second),
    Entry::new(&rings, 8 * second).transition(Transition::fade(second)),
    Entry::new(&flames, 8 * second).transition(Transition::wipe(Edge::Top, second)),
];
let show = Playlist::new(&entries);
```

Transitions: `cut`, `fade`, `fade_through_black`, `wipe(edge, ..)`, `dissolve`
(LEDs switch over in a random order) and `iris` (a circle growing from the middle),
each with an `Easing`. Every entry starts its own clock when its turn comes, so a
wipe or a bouncing comet always starts from the beginning.

**Segments** put different animations on different parts of a strip:

```rust
use ledkit_anim::prelude::*;
use ledkit_layout::Segment;

let left = Comet::new();
let right = Breathe::new();
let parts: [(Segment, &dyn Animation); 2] = [
    (Segment::new(0, 15), &left),
    (Segment::new(15, 15).reversed(), &right),
];
let split = Segments::new(&parts);
```

## Write your own effect

An effect is a struct that implements `Animation`: given an LED's `Position` (its
`x`, `y` and its `index` along the chain) and the `Frame` (the time in
milliseconds, and the `width` and `height` of what is being drawn on), return its
color. It keeps no memory between frames, which is what makes the combining above
work. The `math` module has integer helpers for waves, angles, distances,
repeatable "random" numbers and smooth noise.

```rust
use ledkit_anim::prelude::*;
use ledkit_anim::math;

/// Waves of color rolling along the strip.
struct Waves;

impl Animation for Waves {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let phase = (at.x as u32 * 16 + frame.millis / 8) as u8;
        Rgb::BLUE.scale(math::sine(phase))
    }
}

let mut leds = [Rgb::BLACK; 10];
Waves.render(&mut leds, Time::from_millis(0));
```

## Good to know

- Effects use integer math per LED, so they are quick on chips without a
  floating-point unit. An 8x8 matrix or a few hundred LEDs is easy work.
- "Random" effects (sparkle, rain, flames) are repeatable: the same time always
  gives the same picture.
- `Time` counts milliseconds in a `u32`, so it starts again from 0 after about 49
  days. Effects whose cycle doesn't divide evenly into that jump once when it does.
- To render a `&dyn Animation`, write `(&animation).render(..)`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
