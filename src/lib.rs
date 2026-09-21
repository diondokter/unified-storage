#![cfg_attr(not(feature = "std"), no_std)]

use core::num::NonZero;

#[cfg(feature = "mock")]
pub mod mock;

#[allow(async_fn_in_trait)]
pub trait Storage {
    type Error;

    /// When true, the chip has a uniform size and [Self::layout] will always return the same value.
    ///
    /// The output of this function must be constant so a user can check this once and know the value.
    /// It's not a constant to support runtime discovery of this value by the driver.
    fn uniform_layout(&self) -> bool;
    /// Get the layout for an address.
    /// If [Self::uniform_layout] is true, the same layout is returned for every valid address.
    ///
    /// If the address is out of range, None is returned.
    ///
    /// The output of this function must be constant so a user can check this once and know the value.
    /// It's not a constant to support runtime discovery of this value by the driver.
    ///
    /// The returned storage type must be the same for each address. You're not allowed to mix types in one storage.
    fn layout(&self, addr: u64) -> Option<StorageLayout>;

    /// The value the storage is set to after erasing
    ///
    /// The output of this function must be constant so a user can check this once and know the value.
    /// It's not a constant to support runtime discovery of this value by the driver.
    fn erase_value(&self) -> EraseValue;

    /// The capacity, or highest address (exclusive)
    ///
    /// The output of this function must be constant so a user can check this once and know the value.
    /// It's not a constant to support runtime discovery of this value by the driver.
    fn capacity(&self) -> u64;

    /// Read a slice of data from the storage peripheral, starting the read operation at the given address offset, and reading `bytes.len()` bytes.
    async fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Self::Error>;
    /// Erase the given storage range, clearing all data within [from..to]. The given range will contain all `ERASE_VALUE` bytes afterwards.
    /// If power is lost during erase, contents of the page are undefined.
    ///
    /// The `offset` and `length` must be aligned to full sectors or an error will be returned.
    ///
    /// The use of this function is mandatory for [StorageLayout::Nor] and [StorageLayout::Nand].
    /// For implementations of [StorageLayout::Block] devices, the erase should do a write to emulate everything being erased.
    async fn erase(&mut self, offset: u64, length: u64) -> Result<(), Self::Error>;
    /// Write a slice of data to the storage peripheral, starting the write operation at the given address offset, and writing `bytes.len()` bytes.
    ///
    /// The write offset must be aligned to [`StorageLayout::min_write_size`] and the `bytes.len()` must be a multiple of [`StorageLayout::min_write_size`] or an error will be returned.
    ///
    /// A byte may only be written once before being erased unless specified differently by [StorageLayout::Nor::behavior] for NOR flash or if the layout is [StorageLayout::Block].
    async fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error>;
    /// Wait for the last operation to finish
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageLayout {
    Nor {
        behavior: WriteBehavior,
        /// The minimum write size
        min_write: u32,
        /// The sector size (minimum erase size)
        sector: u32,
    },
    Nand {
        /// Number of partial page programs
        ///
        /// This is the amount of subdivisions of a page. They can be programmed individually, but that must be done sequentially in a page
        nop: NonZero<u8>,
        /// The page size
        page: u32,
        /// The block size (minimum erase size)
        block: u32,
    },
    Block {
        /// The block size (minumum write and erase size)
        size: u32,
    },
    Ram,
}

impl StorageLayout {
    pub const fn min_write_size(&self) -> u32 {
        match self {
            StorageLayout::Nor { min_write, .. } => *min_write,
            StorageLayout::Nand { nop, page, .. } => *page / nop.get() as u32,
            StorageLayout::Block { size } => *size,
            StorageLayout::Ram => 1,
        }
    }

    /// The minimum erase alignment required by the backend in bytes.
    pub const fn erase_size(&self) -> u32 {
        match *self {
            StorageLayout::Nor { sector, .. } => sector,
            StorageLayout::Nand { block, .. } => block,
            StorageLayout::Block { size } => size,
            StorageLayout::Ram => 1,
        }
    }
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
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseValue {
    AllOnes,
    AllZeroes,
    /// Erased data cannot be read and will return an error (possibly due to ECC)
    Indeterminate,
}

impl<T: Storage> Storage for &mut T {
    type Error = T::Error;

    fn uniform_layout(&self) -> bool {
        T::uniform_layout(self)
    }

    fn layout(&self, addr: u64) -> Option<StorageLayout> {
        T::layout(self, addr)
    }

    fn erase_value(&self) -> EraseValue {
        T::erase_value(self)
    }

    fn capacity(&self) -> u64 {
        T::capacity(self)
    }

    fn read(
        &mut self,
        offset: u64,
        bytes: &mut [u8],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        T::read(self, offset, bytes)
    }

    fn erase(&mut self, offset: u64, length: u64) -> impl Future<Output = Result<(), Self::Error>> {
        T::erase(self, offset, length)
    }

    fn write(
        &mut self,
        offset: u64,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        T::write(self, offset, bytes)
    }

    fn flush(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        T::flush(self)
    }
}

pub struct MultiWriteNorFlash<S>(S)
where
    S: embedded_storage_async::nor_flash::MultiwriteNorFlash;

impl<S> Storage for MultiWriteNorFlash<S>
where
    S: embedded_storage_async::nor_flash::MultiwriteNorFlash,
{
    type Error = S::Error;

    fn uniform_layout(&self) -> bool {
        true
    }

    fn layout(&self, addr: u64) -> Option<StorageLayout> {
        if addr >= self.capacity() {
            return None;
        }

        Some(StorageLayout::Nor {
            behavior: WriteBehavior::TwiceAnd,
            min_write: S::WRITE_SIZE as u32,
            sector: S::ERASE_SIZE as u32,
        })
    }

    fn erase_value(&self) -> EraseValue {
        EraseValue::AllOnes
    }

    fn capacity(&self) -> u64 {
        self.0.capacity() as u64
    }

    async fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let offset = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
        self.0.read(offset, bytes).await
    }

    async fn erase(&mut self, offset: u64, length: u64) -> Result<(), Self::Error> {
        let from = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
        let to = (offset + length)
            .try_into()
            .expect("offset + length fits in u32 for embedded-storage");
        self.0.erase(from, to).await
    }

    async fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        let offset = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
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

    fn uniform_layout(&self) -> bool {
        true
    }

    fn layout(&self, addr: u64) -> Option<StorageLayout> {
        if addr >= self.capacity() {
            return None;
        }

        Some(StorageLayout::Nor {
            behavior: WriteBehavior::Once,
            min_write: S::WRITE_SIZE as u32,
            sector: S::ERASE_SIZE as u32,
        })
    }

    fn erase_value(&self) -> EraseValue {
        EraseValue::AllOnes
    }

    fn capacity(&self) -> u64 {
        self.0.capacity() as u64
    }

    async fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let offset = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
        self.0.read(offset, bytes).await
    }

    async fn erase(&mut self, offset: u64, length: u64) -> Result<(), Self::Error> {
        let from = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
        let to = (offset + length)
            .try_into()
            .expect("offset + length fits in u32 for embedded-storage");
        self.0.erase(from, to).await
    }

    async fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        let offset = offset
            .try_into()
            .expect("offset fits in u32 for embedded-storage");
        self.0.write(offset, bytes).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
