use core::{mem::discriminant, range::Range};

use crate::{EraseValue, Storage, StorageLayout, WriteBehavior};

pub struct MockStorage {
    bytes: Vec<Byte>,
    layouts: Vec<(Range<u64>, StorageLayout)>,
    erase_value: EraseValue,
}

impl MockStorage {
    pub fn new(
        layouts: Vec<(Range<u64>, StorageLayout)>,
        erase_value: EraseValue,
        capacity: u64,
    ) -> Self {
        assert!(
            usize::try_from(capacity).is_ok(),
            "capacity must fit in a usize"
        );
        assert!(
            !layouts.is_empty(),
            "layout must contain at least one element"
        );
        let layout_discr = discriminant(&layouts[0].1);
        assert!(
            layouts
                .iter()
                .all(|layout| discriminant(&layout.1) == layout_discr),
            "all layouts must be of the same type"
        );
        // TODO: layouts must not overlap
        // TODO: layouts must cover address ranges 0..capacity

        let layout = layouts[0].1;

        Self {
            bytes: vec![
                Byte {
                    value: 0,
                    erased: true,
                    writes_left: writes_from_layout(layout)
                };
                capacity as usize
            ],
            layouts,
            erase_value,
        }
    }
}

impl Storage for MockStorage {
    type Error = MockError;

    fn uniform_layout(&self) -> bool {
        self.layouts.len() == 1
    }

    fn layout(&self, addr: u64) -> Option<StorageLayout> {
        self.layouts
            .iter()
            .find_map(|(range, layout)| range.contains(&addr).then_some(*layout))
    }

    fn erase_value(&self) -> EraseValue {
        self.erase_value
    }

    fn capacity(&self) -> u64 {
        self.bytes.len() as u64
    }

    async fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    async fn erase(&mut self, offset: u64, length: u64) -> Result<(), Self::Error> {
        if offset + length > self.capacity() {
            return Err(MockError::OutOfBounds);
        }

        let mut current_offset = offset;

        while current_offset != offset + length {
            let length_left = offset + length - current_offset;
            let local_layout = self.layout(offset).ok_or(MockError::OutOfBounds)?;
            if !current_offset.is_multiple_of(local_layout.erase_size().into()) {
                return Err(MockError::Unaligned);
            }
            if length_left < local_layout.erase_size().into() {
                return Err(MockError::Unaligned);
            }

            for byte in
                &mut self.bytes[current_offset as usize..][..local_layout.erase_size() as usize]
            {
                byte.erased = true;
                byte.writes_left = writes_from_layout(local_layout);
            }

            current_offset += local_layout.erase_size() as u64;
        }

        Ok(())
    }

    async fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // No-op
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockError {
    OutOfBounds,
    Unaligned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Byte {
    value: u8,
    erased: bool,
    writes_left: Option<u8>,
}

fn writes_from_layout(layout: StorageLayout) -> Option<u8> {
    match layout {
        StorageLayout::Nor {
            behavior: WriteBehavior::Once,
            ..
        } => Some(1),
        StorageLayout::Nor {
            behavior: WriteBehavior::TwiceAnd,
            ..
        } => Some(2),
        StorageLayout::Nor {
            behavior: WriteBehavior::TwiceSecondZero,
            ..
        } => Some(2),
        StorageLayout::Nor {
            behavior: WriteBehavior::InfiniteAnd,
            ..
        } => None,
        StorageLayout::Nand { .. } => Some(1),
        StorageLayout::Block { .. } => None,
        StorageLayout::Ram => None,
    }
}
