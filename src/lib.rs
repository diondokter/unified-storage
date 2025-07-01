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
    /// The reliability of the storage
    const RELIABILITY: Reliability;

    /// The capacity, or highest address (exclusive)
    fn capacity(&self) -> usize;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;
    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

/// The way multiple writes act on the storage
#[non_exhaustive]
pub enum WriteBehavior {
    /// The memory can be written once and must then be erased.
    /// It's undefined what happens if [Storage::write] is called more than once without erasing.
    Once,
    /// The memory can be written twice and must then be erased.
    /// The second write must be all 0's (likely due to ECC).
    /// It's undefined what happens if [Storage::write] is called more than twice without erasing or when the second write is not 0's.
    TwiceSecondZero,
    /// The memory can be written infinitely without erasing.
    /// The new write value will be AND'ed with the existing value.
    InfiniteAnd,
    /// The memory can be written infinitely without erasing.
    /// The written value is also what can be read back. (No AND happening)
    InfiniteDirect,
}

/// The reliability of the storage medium
pub enum Reliability {
    /// The storage is always reliable (e.g. ECC RAM)
    Good,
    /// The storage is mostly reliable, though random bitflips can happen (e.g. non-ECC RAM)
    Medium,
    /// The storage is reliably by design. The storage can get bad at the end of life (e.g. ECC NOR)
    GoodDegrading,
    /// The storage is mostly reliable by design, though random bitflips can happen. The storage can get bad at the end of life (e.g. non-ECC NOR & ECC NAND)
    MediumDegrading,
    /// The storage is always unreliable and it behooves the user to add error correction (e.g. non-ECC NAND)
    Bad,
}