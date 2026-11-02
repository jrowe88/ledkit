//! The PIO program and the state machine set-up both outputs share.

use ledkit_core::encode::PioTiming;
use ledkit_core::{Chip, ClocklessTiming, ResetGap};

use crate::clock::{drain_us, FrameClock, MicrosecondClock};
use crate::hal::gpio::{AnyPin, SpecificPin};
use crate::hal::pio::{
    Buffers, PIOBuilder, PIOExt, PinDir, PinState, Running, Rx, ShiftDirection, StateMachine,
    StateMachineIndex, Tx, UninitStateMachine, PIO,
};
use crate::Error;

/// The longest step of the program, in PIO cycles. Each step is one instruction,
/// whose delay field holds 0 to 15 when one bit is used for side-set.
const MAX_STEP_CYCLES: u32 = 16;

/// The largest whole clock divider of a state machine (16 bits).
const MAX_DIVIDER: u32 = 65_535;

/// How many wire bytes go in one FIFO word: one LED's worth, so a frame is
/// exactly one word per LED. A compile error for chips with more than four
/// channels, which don't fit a 32-bit word.
pub(crate) const fn bytes_per_word<C: Chip>() -> usize {
    let channels = C::ORDER.len();
    assert!(
        channels >= 1 && channels <= 4,
        "ledkit-rp sends one LED per 32-bit word, so a chip may have at most 4 channels"
    );
    channels
}

/// The state machine timing for chip `C` at system clock `sys_hz`.
pub(crate) fn timing_for<C: Chip>(sys_hz: u32) -> Result<PioTiming, Error> {
    // Evaluated at compile time: a clocked chip (a DotStar) fails the build here.
    let timing = const { ClocklessTiming::of::<C>() };
    Ok(timing.pio_timing(sys_hz, MAX_STEP_CYCLES, MAX_DIVIDER)?)
}

/// Assembles the program. Every bit takes `high + data + low` cycles:
///
/// ```text
/// .side_set 1
/// .wrap_target
/// bitloop:
///     out x, 1        side 0 [low - 1]   ; next bit; the line is low (also while waiting for data)
///     jmp !x do_zero  side 1 [high - 1]  ; every bit starts high
///     jmp bitloop     side 1 [data - 1]  ; a 1 stays high
/// do_zero:
///     nop             side 0 [data - 1]  ; a 0 goes low
/// .wrap
/// ```
///
/// This is the Raspberry Pi `ws2812.pio` program, with its three delays
/// computed for the chip instead of fixed.
fn program(ticks: &PioTiming) -> pio::Program<{ pio::RP2040_MAX_PROGRAM_SIZE }> {
    let delay = |cycles: u32| (cycles - 1) as u8;
    let side_set = pio::SideSet::new(false, 1, false);
    let mut a = pio::Assembler::<{ pio::RP2040_MAX_PROGRAM_SIZE }>::new_with_side_set(side_set);
    let mut bitloop = a.label();
    let mut do_zero = a.label();
    let mut wrap_source = a.label();
    a.bind(&mut bitloop);
    a.out_with_delay_and_side_set(pio::OutDestination::X, 1, delay(ticks.low), 0);
    a.jmp_with_delay_and_side_set(
        pio::JmpCondition::XIsZero,
        &mut do_zero,
        delay(ticks.high),
        1,
    );
    a.jmp_with_delay_and_side_set(
        pio::JmpCondition::Always,
        &mut bitloop,
        delay(ticks.data),
        1,
    );
    a.bind(&mut do_zero);
    a.nop_with_delay_and_side_set(delay(ticks.data), 0);
    a.bind(&mut wrap_source);
    a.assemble_with_wrap(wrap_source, bitloop)
}

/// A running state machine sending one LED per word on one pin.
pub(crate) struct Machine<P: PIOExt, SM: StateMachineIndex, I: AnyPin> {
    pub(crate) sm: StateMachine<(P, SM), Running>,
    pub(crate) rx: Rx<(P, SM)>,
    pub(crate) pin: I,
}

/// Installs the program for chip `C` on `pio`, sets up `sm` to send it on `pin`
/// and starts it. Returns the machine, its FIFO and the frame clock.
pub(crate) fn start<C, P, SM, I, T>(
    pio: &mut PIO<P>,
    sm: UninitStateMachine<(P, SM)>,
    pin: I,
    sys_hz: u32,
    clock: T,
) -> Result<(Machine<P, SM, I>, Tx<(P, SM)>, FrameClock<T>), Error>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    let bytes = const { bytes_per_word::<C>() };
    let ticks = timing_for::<C>(sys_hz)?;
    let installed = pio
        .install(&program(&ticks))
        .map_err(|_| Error::NoProgramSpace)?;

    let pin: SpecificPin<I> = pin.into();
    let pin_number = pin.id().num;
    let bits_per_word = 8 * bytes as u32;
    let (mut sm, rx, tx) = PIOBuilder::from_installed_program(installed)
        // The receive FIFO is unused; joining gives the transmit FIFO 8 words.
        .buffers(Buffers::OnlyTx)
        .side_set_pin_base(pin_number)
        // Most significant bit first, as the LEDs read it.
        .out_shift_direction(ShiftDirection::Left)
        .autopull(true)
        // 32 is written as 0 in the 5-bit field.
        .pull_threshold((bits_per_word % 32) as u8)
        // Whole dividers only: a fractional one makes the pulses jitter.
        .clock_divisor_fixed_point(ticks.divider as u16, 0)
        .build(sm);
    // Drive the pin low before the program starts, so the LEDs see a clean
    // reset rather than a floating line.
    sm.set_pins([(pin_number, PinState::Low)]);
    sm.set_pindirs([(pin_number, PinDir::Output)]);
    let sm = sm.start();

    let frame = FrameClock::new(
        clock,
        ResetGap::of::<C>(),
        drain_us(bits_per_word, ticks.period(), ticks.divider, sys_hz),
    );
    Ok((
        Machine {
            sm,
            rx,
            pin: I::from(pin),
        },
        tx,
        frame,
    ))
}

impl<P: PIOExt, SM: StateMachineIndex, I: AnyPin> Machine<P, SM, I> {
    /// Stops the state machine, removes the program and hands back the state
    /// machine and the pin.
    pub(crate) fn release(
        self,
        pio: &mut PIO<P>,
        tx: Tx<(P, SM)>,
    ) -> (UninitStateMachine<(P, SM)>, I) {
        let (sm, program) = self.sm.stop().uninit(self.rx, tx);
        pio.uninstall(program);
        (sm, self.pin)
    }
}
