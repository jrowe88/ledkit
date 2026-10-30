//! Pretend SPI buses for the tests in this crate.

use core::fmt;
use embedded_hal::spi::{ErrorKind, ErrorType};

/// The error the fake buses report when told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeError;

impl fmt::Display for FakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fake bus failure")
    }
}

impl embedded_hal::spi::Error for FakeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

/// A blocking SPI bus that remembers everything written to it.
#[derive(Debug, Default)]
pub struct FakeBus {
    /// Everything written, joined together.
    pub bytes: Vec<u8>,
    /// How many times `write` was called.
    pub writes: usize,
    /// How many times `flush` was called.
    pub flushes: usize,
    /// When set, `write` fails with [`FakeError`].
    pub fail: bool,
}

impl ErrorType for FakeBus {
    type Error = FakeError;
}

impl embedded_hal::spi::SpiBus<u8> for FakeBus {
    fn read(&mut self, _words: &mut [u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    fn write(&mut self, words: &[u8]) -> Result<(), FakeError> {
        if self.fail {
            return Err(FakeError);
        }
        self.bytes.extend_from_slice(words);
        self.writes += 1;
        Ok(())
    }
    fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    fn flush(&mut self) -> Result<(), FakeError> {
        self.flushes += 1;
        Ok(())
    }
}

/// Like [`FakeBus`], but for the async traits.
#[derive(Debug, Default)]
pub struct FakeAsyncBus {
    /// Everything written, joined together.
    pub bytes: Vec<u8>,
    /// How many times `write` was called.
    pub writes: usize,
    /// How many times `flush` was called.
    pub flushes: usize,
    /// When set, `write` fails with [`FakeError`].
    pub fail: bool,
}

impl ErrorType for FakeAsyncBus {
    type Error = FakeError;
}

#[allow(clippy::unused_async_trait_impl)] // the trait requires async fns
impl embedded_hal_async::spi::SpiBus<u8> for FakeAsyncBus {
    async fn read(&mut self, _words: &mut [u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    async fn write(&mut self, words: &[u8]) -> Result<(), FakeError> {
        if self.fail {
            return Err(FakeError);
        }
        self.bytes.extend_from_slice(words);
        self.writes += 1;
        Ok(())
    }
    async fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    async fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), FakeError> {
        unimplemented!()
    }
    async fn flush(&mut self) -> Result<(), FakeError> {
        self.flushes += 1;
        Ok(())
    }
}
