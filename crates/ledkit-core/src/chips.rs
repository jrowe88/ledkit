//! The LED chip catalogue.
//!
//! Find the name printed on your product (or its datasheet) and use the
//! matching type.
//!
//! | Printed on the product | Type |
//! |---|---|
//! | NeoPixel strip, ring, matrix, "WS2812B" | [`Ws2812b`] |
//! | "WS2812" (older, 5 V) | [`Ws2812`] |
//! | "WS2813" (backup data line) | [`Ws2813`] |
//! | "WS2815", 12 V strip | [`Ws2815`] |
//! | "WS2811", 12 V pixel strings or bullet lights | [`Ws2811`] |
//! | "WS2811" at 400 kHz (very old strings) | [`Ws2811Slow`] |
//! | "SK6812", "SK6812 Mini", "SK6805" | [`Sk6812`] (aliases [`Sk6812Mini`], [`Sk6805`]) |
//! | "SK6812 RGBW", RGBW NeoPixel | [`Sk6812Rgbw`] |
//! | "APA106", 8 mm through-hole pixels | [`Apa106`] |
//! | "DotStar", "APA102", "APA102C" | [`Apa102`] |
//! | "SK9822" (APA102 clone) | [`Sk9822`] |
//! | SparkFun LuMini | [`Apa102_2020`] (alias of [`Apa102`]) |

use crate::{ChannelOrder, Chip, ClockedFormat, ClocklessTiming, Phase, Protocol, Rgb, Rgbw};

macro_rules! clockless_chip {
    (
        $(#[$doc:meta])*
        $name:ident, $label:literal, $pixel:ty, $order:ident,
        [$zh:expr, $zl:expr, $oh:expr, $ol:expr], $reset:expr
    ) => {
        $(#[$doc])*
        ///
        /// # Example
        ///
        /// ```
        #[doc = concat!("use ledkit_core::{Chip, Protocol, ", stringify!($name), "};")]
        ///
        #[doc = concat!("assert_eq!(", stringify!($name), "::NAME, ", stringify!($label), ");")]
        #[doc = concat!("assert!(matches!(", stringify!($name), "::PROTOCOL, Protocol::Clockless(_)));")]
        /// ```
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $name;

        impl Chip for $name {
            type Pixel = $pixel;
            const NAME: &'static str = $label;
            const ORDER: ChannelOrder = ChannelOrder::$order;
            const PROTOCOL: Protocol = Protocol::Clockless(ClocklessTiming {
                zero_high: $zh,
                zero_low: $zl,
                one_high: $oh,
                one_low: $ol,
                reset_us: $reset,
            });
        }
    };
}

macro_rules! clocked_chip {
    ($(#[$doc:meta])* $name:ident, $label:literal) => {
        $(#[$doc])*
        ///
        /// # Example
        ///
        /// ```
        #[doc = concat!("use ledkit_core::{Chip, Protocol, ", stringify!($name), "};")]
        ///
        #[doc = concat!("assert_eq!(", stringify!($name), "::NAME, ", stringify!($label), ");")]
        #[doc = concat!("assert!(matches!(", stringify!($name), "::PROTOCOL, Protocol::Clocked(_)));")]
        /// ```
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $name;

        impl Chip for $name {
            type Pixel = Rgb;
            const NAME: &'static str = $label;
            const ORDER: ChannelOrder = ChannelOrder::BGR;
            const PROTOCOL: Protocol = Protocol::Clocked(ClockedFormat::Apa102);
        }
    };
}

const fn ph(typical: u32, min: u32, max: u32) -> Phase {
    Phase::new(typical, min, max)
}

clockless_chip!(
    /// The original WS2812: the chip in early NeoPixel strips and sticks.
    Ws2812, "WS2812", Rgb, GRB,
    [ph(350, 200, 500), ph(800, 650, 5000), ph(700, 550, 850), ph(600, 450, 5000)], 50
);

clockless_chip!(
    /// The WS2812B: the chip in most NeoPixel strips, rings and matrices sold today.
    Ws2812b, "WS2812B", Rgb, GRB,
    [ph(400, 220, 450), ph(900, 580, 5000), ph(800, 580, 1000), ph(600, 450, 5000)], 280
);

clockless_chip!(
    /// The WS2813: like the WS2812B but with a backup data line, found in
    /// more robust 5 V strips.
    Ws2813, "WS2813", Rgb, GRB,
    [ph(375, 300, 450), ph(875, 300, 5000), ph(875, 750, 1000), ph(375, 300, 5000)], 300
);

clockless_chip!(
    /// The WS2815: the 12 V cousin of the WS2813, found in 12 V addressable strips.
    ///
    /// It is a 12 V chip, but its `POWER` uses the 5 V model as an approximation.
    Ws2815, "WS2815", Rgb, GRB,
    [ph(375, 300, 450), ph(875, 300, 5000), ph(875, 750, 1000), ph(375, 300, 5000)], 300
);

clockless_chip!(
    /// The WS2811: the driver chip in 12 V pixel strings, bullet pixels and
    /// some cheap strips (one chip drives three LEDs).
    Ws2811, "WS2811", Rgb, RGB,
    [ph(250, 100, 400), ph(1000, 850, 5000), ph(600, 450, 750), ph(650, 500, 5000)], 280
);

clockless_chip!(
    /// A WS2811 running in its slow 400 kHz mode, found in very old pixel strings.
    Ws2811Slow, "WS2811 (400 kHz)", Rgb, RGB,
    [ph(500, 350, 650), ph(2000, 1850, 10000), ph(1200, 1050, 1350), ph(1300, 1150, 10000)], 280
);

clockless_chip!(
    /// The SK6812: a WS2812B look-alike found in NeoPixel-style strips,
    /// minis and tiny LEDs.
    Sk6812, "SK6812", Rgb, GRB,
    [ph(300, 150, 450), ph(900, 750, 5000), ph(600, 450, 750), ph(600, 450, 5000)], 80
);

clockless_chip!(
    /// The SK6812 RGBW: RGBW NeoPixels with a separate white LED.
    Sk6812Rgbw, "SK6812 RGBW", Rgbw, GRBW,
    [ph(300, 150, 450), ph(900, 750, 5000), ph(600, 450, 750), ph(600, 450, 5000)], 80
);

clockless_chip!(
    /// The APA106: the chip in 8 mm through-hole RGB pixels.
    Apa106, "APA106", Rgb, RGB,
    [ph(350, 200, 500), ph(1360, 1210, 5000), ph(1360, 1210, 1510), ph(350, 200, 5000)], 50
);

clocked_chip!(
    /// The APA102: the chip in DotStar strips.
    Apa102, "APA102"
);

clocked_chip!(
    /// The SK9822: an APA102 clone found in cheaper DotStar-style strips.
    Sk9822, "SK9822"
);

/// The SK6812 Mini, found in compact NeoPixel-style boards. Same as [`Sk6812`].
pub type Sk6812Mini = Sk6812;

/// The SK6805, a very small NeoPixel-style LED. Same as [`Sk6812`].
pub type Sk6805 = Sk6812;

/// The tiny APA102-2020 LEDs on the SparkFun LuMini. Same as [`Apa102`].
#[allow(non_camel_case_types)]
pub type Apa102_2020 = Apa102;

#[cfg(test)]
mod tests {
    use crate::*;

    fn clockless<C: Chip>() -> ClocklessTiming {
        match C::PROTOCOL {
            Protocol::Clockless(t) => t,
            Protocol::Clocked(_) => panic!("{} is clocked", C::NAME),
        }
    }

    #[test]
    fn ws2812b_timing() {
        let t = clockless::<Ws2812b>();
        assert_eq!(
            (t.zero_high.typical_ns, t.one_high.typical_ns, t.reset_us),
            (400, 800, 280)
        );
        assert!(t.zero_high.contains(450) && !t.zero_high.contains(451));
    }

    #[test]
    fn orders() {
        assert_eq!(Ws2812b::ORDER, ChannelOrder::GRB);
        assert_eq!(Ws2811::ORDER, ChannelOrder::RGB);
        assert_eq!(Apa102::ORDER, ChannelOrder::BGR);
        assert_eq!(Sk6812Rgbw::ORDER, ChannelOrder::GRBW);
    }

    #[test]
    fn dotstars_are_clocked() {
        assert!(matches!(
            Apa102::PROTOCOL,
            Protocol::Clocked(ClockedFormat::Apa102)
        ));
        assert!(matches!(
            Sk9822::PROTOCOL,
            Protocol::Clocked(ClockedFormat::Apa102)
        ));
    }

    #[test]
    fn rgbw_chip_uses_rgbw_pixels() {
        assert_eq!(<<Sk6812Rgbw as Chip>::Pixel as Pixel>::CHANNELS, 4);
    }

    #[test]
    fn ws2815_matches_ws2813() {
        assert_eq!(clockless::<Ws2815>(), clockless::<Ws2813>());
    }

    #[test]
    fn every_typical_value_is_inside_its_window() {
        for t in [
            clockless::<Ws2812>(),
            clockless::<Ws2812b>(),
            clockless::<Ws2813>(),
            clockless::<Ws2811>(),
            clockless::<Ws2811Slow>(),
            clockless::<Sk6812>(),
            clockless::<Apa106>(),
        ] {
            for p in [t.zero_high, t.zero_low, t.one_high, t.one_low] {
                assert!(p.contains(p.typical_ns), "{p:?}");
            }
        }
    }

    /// One row of the catalogue table: `(typical, min, max)` per phase.
    type Row = (
        &'static str,
        ClocklessTiming,
        ChannelOrder,
        usize,
        [(u32, u32, u32); 4],
        u32,
    );

    fn row<C: Chip>(phases: [(u32, u32, u32); 4], reset_us: u32) -> Row {
        (
            C::NAME,
            clockless::<C>(),
            C::ORDER,
            <C::Pixel as Pixel>::CHANNELS,
            phases,
            reset_us,
        )
    }

    #[test]
    fn clockless_catalogue_matches_the_table() {
        use ChannelOrder as O;
        let ws2813 = [
            (375, 300, 450),
            (875, 300, 5000),
            (875, 750, 1000),
            (375, 300, 5000),
        ];
        let sk6812 = [
            (300, 150, 450),
            (900, 750, 5000),
            (600, 450, 750),
            (600, 450, 5000),
        ];
        #[rustfmt::skip]
        let table: [(Row, ChannelOrder, usize); 9] = [
            (row::<Ws2812>([(350, 200, 500), (800, 650, 5000), (700, 550, 850), (600, 450, 5000)], 50), O::GRB, 3),
            (row::<Ws2812b>([(400, 220, 450), (900, 580, 5000), (800, 580, 1000), (600, 450, 5000)], 280), O::GRB, 3),
            (row::<Ws2813>(ws2813, 300), O::GRB, 3),
            (row::<Ws2815>(ws2813, 300), O::GRB, 3),
            (row::<Ws2811>([(250, 100, 400), (1000, 850, 5000), (600, 450, 750), (650, 500, 5000)], 280), O::RGB, 3),
            (row::<Ws2811Slow>([(500, 350, 650), (2000, 1850, 10000), (1200, 1050, 1350), (1300, 1150, 10000)], 280), O::RGB, 3),
            (row::<Sk6812>(sk6812, 80), O::GRB, 3),
            (row::<Sk6812Rgbw>(sk6812, 80), O::GRBW, 4),
            (row::<Apa106>([(350, 200, 500), (1360, 1210, 5000), (1360, 1210, 1510), (350, 200, 5000)], 50), O::RGB, 3),
        ];
        for ((name, t, order, channels, phases, reset_us), want_order, want_channels) in table {
            let got = [t.zero_high, t.zero_low, t.one_high, t.one_low]
                .map(|p| (p.typical_ns, p.min_ns, p.max_ns));
            assert_eq!(got, phases, "{name} phases");
            assert_eq!(t.reset_us, reset_us, "{name} reset");
            assert_eq!(order, want_order, "{name} order");
            assert_eq!(channels, want_channels, "{name} pixel channels");
            assert_eq!(order.len(), channels, "{name} order length");
        }
    }

    #[test]
    fn clocked_catalogue_matches_the_table() {
        for (name, protocol, order, channels) in [
            (
                Apa102::NAME,
                Apa102::PROTOCOL,
                Apa102::ORDER,
                <Apa102 as Chip>::Pixel::CHANNELS,
            ),
            (
                Sk9822::NAME,
                Sk9822::PROTOCOL,
                Sk9822::ORDER,
                <Sk9822 as Chip>::Pixel::CHANNELS,
            ),
        ] {
            assert_eq!(protocol, Protocol::Clocked(ClockedFormat::Apa102), "{name}");
            assert_eq!(order, ChannelOrder::BGR, "{name}");
            assert_eq!(channels, 3, "{name}");
        }
    }

    #[test]
    fn phase_window_includes_both_edges_only() {
        let p = Phase::new(400, 220, 450);
        assert!(p.contains(220) && p.contains(450));
        assert!(!p.contains(219) && !p.contains(451));
    }

    #[test]
    fn default_power_model() {
        assert_eq!(Ws2812b::POWER, PowerModel::TYPICAL_5V);
    }
}
