//! Pretend serial ports and delays for the tests.

use core::cell::RefCell;
use core::fmt;
use std::rc::Rc;

/// Something a fake port or delay did, in the order it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A successful `write` of this many bytes.
    Write(usize),
    /// A `flush`.
    Flush,
    /// A wait of this many microseconds.
    Wait(u32),
}

/// A log the fakes share, so tests can check the order of events.
pub type Log = Rc<RefCell<Vec<Event>>>;

/// A port and a delay that write to the same [`Log`].
pub fn pair() -> (FakePort, FakeDelay, Log) {
    let log = Log::default();
    let port = FakePort {
        log: log.clone(),
        ..FakePort::default()
    };
    let delay = FakeDelay {
        log: log.clone(),
        ..FakeDelay::default()
    };
    (port, delay, log)
}

/// The error a [`FakePort`] returns when told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeError;

impl fmt::Display for FakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fake port failure")
    }
}

impl core::error::Error for FakeError {}

impl embedded_io::Error for FakeError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

/// A serial port that records what it is sent.
#[derive(Debug)]
pub struct FakePort {
    /// Every byte written, in order.
    pub bytes: Vec<u8>,
    /// How many times `flush` was called.
    pub flushes: usize,
    /// The most bytes one `write` call accepts.
    pub max_per_write: usize,
    /// If set, the `write` call with this index (counting every call from 0) fails.
    pub fail_on_write: Option<usize>,
    /// How many `write` calls have been made, failed ones included.
    pub writes: usize,
    /// Where the port records what it did.
    pub log: Log,
}

impl Default for FakePort {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            flushes: 0,
            max_per_write: usize::MAX,
            fail_on_write: None,
            writes: 0,
            log: Log::default(),
        }
    }
}

impl FakePort {
    fn accept(&mut self, buf: &[u8]) -> Result<usize, FakeError> {
        let call = self.writes;
        self.writes += 1;
        if self.fail_on_write == Some(call) {
            return Err(FakeError);
        }
        let n = buf.len().min(self.max_per_write);
        self.bytes.extend_from_slice(&buf[..n]);
        self.log.borrow_mut().push(Event::Write(n));
        Ok(n)
    }
}

impl embedded_io::ErrorType for FakePort {
    type Error = FakeError;
}

impl embedded_io::Write for FakePort {
    fn write(&mut self, buf: &[u8]) -> Result<usize, FakeError> {
        self.accept(buf)
    }
    fn flush(&mut self) -> Result<(), FakeError> {
        self.flushes += 1;
        self.log.borrow_mut().push(Event::Flush);
        Ok(())
    }
}

#[allow(clippy::unused_async_trait_impl)] // the trait requires async fns
impl embedded_io_async::Write for FakePort {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, FakeError> {
        self.accept(buf)
    }
    async fn flush(&mut self) -> Result<(), FakeError> {
        self.flushes += 1;
        self.log.borrow_mut().push(Event::Flush);
        Ok(())
    }
}

/// A delay that records each wait instead of waiting.
#[derive(Debug, Default)]
pub struct FakeDelay {
    /// Every wait, in microseconds.
    pub waits_us: Vec<u32>,
    /// Where the delay records its waits.
    pub log: Log,
}

impl FakeDelay {
    fn wait(&mut self, us: u32) {
        self.waits_us.push(us);
        self.log.borrow_mut().push(Event::Wait(us));
    }
}

impl embedded_hal::delay::DelayNs for FakeDelay {
    fn delay_ns(&mut self, ns: u32) {
        self.wait(ns / 1000);
    }
    fn delay_us(&mut self, us: u32) {
        self.wait(us);
    }
}

#[allow(clippy::unused_async_trait_impl)] // the trait requires async fns
impl embedded_hal_async::delay::DelayNs for FakeDelay {
    async fn delay_ns(&mut self, ns: u32) {
        self.wait(ns / 1000);
    }
    async fn delay_us(&mut self, us: u32) {
        self.wait(us);
    }
}
