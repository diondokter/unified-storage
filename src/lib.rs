#![no_std]

#[allow(async_fn_in_trait)]
pub trait Storage {
    type Error;

    /// The smallest size that can be read from the storage.
    ///
    /// This should be 1, unless there's really no way to read one byte.
    /// Ideally the driver can emulate single-byte reads if the hardware doesn't support it.
    const READ_SIZE: usize;
    /// The smallest size that can be written to the storage.
    const WRITE_SIZE: usize;
    /// The smallest size that can be erased from the storage.
    const ERASE_SIZE: usize;

    /// The value the storage is set to after erasing
    ///
    /// Typically one of: 0xFF or 0x00
    const ERASE_VALUE: u8;
    /// How successive writes behave
    const WRITE_BEHAVIOR: WriteBehavior;

    /// The capacity, or highest address (exclusive)
    fn capacity(&self) -> usize;

    /// Read a slice of data from the storage peripheral, starting the read operation at the given address offset, and reading `bytes.len()` bytes.
    ///
    /// The read offset must be aligned to `READ_SIZE` and the `bytes.len()` must be a multiple of `READ_SIZE` or an error will be returned.
    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;
    /// Erase the given storage range, clearing all data within [from..to]. The given range will contain all `ERASE_VALUE` bytes afterwards.
    /// If power is lost during erase, contents of the page are undefined.
    ///
    /// The `from` and `to` must be aligned to `ERASE_SIZE` or an error will be returned.
    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
    /// Write a slice of data to the storage peripheral, starting the write operation at the given address offset, and writing `bytes.len()` bytes.
    ///
    /// The write offset must be aligned to `WRITE_SIZE` and the `bytes.len()` must be a multiple of `WRITE_SIZE` or an error will be returned.
    /// The operation follows the behavior as specified by `WRITE_BEHAVIOR`.
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
    /// Wait for the last operation to finish
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

/// The way multiple writes act on the storage
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBehavior {
    /// The memory can be written once and must then be erased.
    /// It's undefined what happens if [Storage::write] is called more than once without erasing.
    Once,
    /// The memory can be written twice and must then be erased.
    /// The second write must be all 0's (likely due to ECC).
    /// It's undefined what happens if [Storage::write] is called more than twice without erasing or when the second write is not 0's.
    TwiceSecondZero,
    /// The memory can be written twice and must then be erased.
    /// It's undefined what happens if [Storage::write] is called more than twice without erasing.
    /// The new write value will be AND'ed with the existing value.
    TwiceAnd,
    /// The memory can be written infinitely without erasing.
    /// The new write value will be AND'ed with the existing value.
    InfiniteAnd,
    /// The memory can be written infinitely without erasing.
    /// The written value is also what can be read back. (No AND happening)
    InfiniteDirect,
}

pub struct MultiWriteNorFlash<S>(S)
where
    S: embedded_storage_async::nor_flash::MultiwriteNorFlash;

impl<S> Storage for MultiWriteNorFlash<S>
where
    S: embedded_storage_async::nor_flash::MultiwriteNorFlash,
{
    type Error = S::Error;

    const READ_SIZE: usize = S::READ_SIZE;
    const WRITE_SIZE: usize = S::WRITE_SIZE;
    const ERASE_SIZE: usize = S::ERASE_SIZE;

    const ERASE_VALUE: u8 = 0xFF;
    const WRITE_BEHAVIOR: WriteBehavior = WriteBehavior::TwiceAnd;

    fn capacity(&self) -> usize {
        self.0.capacity()
    }

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.0.read(offset, bytes).await
    }

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.0.erase(from, to).await
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write(offset, bytes).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct NorFlash<S>(S)
where
    S: embedded_storage_async::nor_flash::NorFlash;

impl<S> Storage for NorFlash<S>
where
    S: embedded_storage_async::nor_flash::NorFlash,
{
    type Error = S::Error;

    const READ_SIZE: usize = S::READ_SIZE;
    const WRITE_SIZE: usize = S::WRITE_SIZE;
    const ERASE_SIZE: usize = S::ERASE_SIZE;

    const ERASE_VALUE: u8 = 0xFF;
    const WRITE_BEHAVIOR: WriteBehavior = WriteBehavior::Once;

    fn capacity(&self) -> usize {
        self.0.capacity()
    }

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.0.read(offset, bytes).await
    }

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.0.erase(from, to).await
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write(offset, bytes).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
