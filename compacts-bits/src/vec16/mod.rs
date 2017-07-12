#[macro_use]
mod delegate;
mod rank_select;
mod pairwise;

#[cfg(test)]
mod tests;

use std::fmt;
use block;

#[derive(Clone)]
pub(crate) enum Vec16 {
    Seq16(block::Seq16),
    Seq64(block::Seq64),
    Rle16(block::Rle16),
}
use self::Vec16::*;

impl fmt::Debug for Vec16 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Seq16(ref d) => write!(fmt, "{:?}", d),
            Seq64(ref d) => write!(fmt, "{:?}", d),
            Rle16(ref d) => write!(fmt, "{:?}", d),
        }
    }
}

impl Default for Vec16 {
    fn default() -> Self {
        // Seq16(block::Seq16::new())
        Seq64(block::Seq64::new())
        // Rle16(block::Rle16::new())
    }
}

const SEQ16_THRESHOLD: usize = 4096; // 4096 * 16 == 65536
const SEQ64_THRESHOLD: usize = 1024; // 1024 * 64 == 65536

impl Vec16 {
    pub const CAPACITY: u32 = block::CAPACITY as u32;

    pub fn new() -> Self {
        Vec16::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    // fn as_vec16(&mut self) {
    //     *self = match *self {
    //         Seq64(ref b) => Seq16(block::Seq16::from(b)),
    //         Rle16(ref b) => Seq16(block::Seq16::from(b)),
    //         _ => unreachable!("already vec16"),
    //     }
    // }

    fn as_seq64(&mut self) {
        *self = match *self {
            Seq16(ref b) => Seq64(block::Seq64::from(b)),
            Rle16(ref b) => Seq64(block::Seq64::from(b)),
            _ => unreachable!("already vec64"),
        }
    }

    // fn as_rle16(&mut self) {
    //     *self = match *self {
    //         Seq16(ref b) => Rle16(block::Rle16::from(b)),
    //         Seq64(ref b) => Rle16(block::Rle16::from(b)),
    //         _ => unreachable!("already rle16"),
    //     }
    // }

    /// May convert to more efficient block representaions.
    /// This may consume many time and resource. So, don't call too much.
    pub fn optimize(&mut self) {
        let new_block = match *self {
            Seq16(ref old) => {
                let mem_in_seq16 = old.mem_size();
                let mem_in_seq64 = block::Seq64::size_in_bytes(SEQ64_THRESHOLD);
                let mem_in_rle16 = block::Rle16::size_in_bytes(old.count_rle());

                if mem_in_rle16 <= ::std::cmp::min(mem_in_seq64, mem_in_seq16) {
                    Some(Rle16(block::Rle16::from(old)))
                } else if self.count_ones() as usize <= SEQ16_THRESHOLD {
                    None
                } else {
                    Some(Seq64(block::Seq64::from(old)))
                }
            }

            Seq64(ref old) => {
                let mem_in_seq16 = block::Seq16::size_in_bytes(old.count_ones() as usize);
                let mem_in_seq64 = old.mem_size();
                let mem_in_rle16 = block::Rle16::size_in_bytes(old.count_rle());

                if mem_in_rle16 <= ::std::cmp::min(mem_in_seq64, mem_in_seq16) {
                    Some(Rle16(block::Rle16::from(old)))
                } else if self.count_ones() as usize <= SEQ16_THRESHOLD {
                    Some(Seq16(block::Seq16::from(old)))
                } else {
                    None
                }
            }

            Rle16(ref old) => {
                let mem_in_seq16 = block::Seq16::size_in_bytes(old.count_ones() as usize);
                let mem_in_seq64 = block::Seq64::size_in_bytes(SEQ64_THRESHOLD);
                let mem_in_rle16 = old.mem_size();

                if mem_in_rle16 <= ::std::cmp::min(mem_in_seq64, mem_in_seq16) {
                    None
                } else if self.count_ones() as usize <= SEQ16_THRESHOLD {
                    Some(Seq16(block::Seq16::from(old)))
                } else {
                    Some(Seq64(block::Seq64::from(old)))
                }
            }
        };
        if let Some(block) = new_block {
            *self = block;
        }
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
impl Vec16 {
    pub fn mem_size(&self)    -> usize { delegate!(ref self, mem_size)    }
    pub fn count_ones(&self)  -> u32   { delegate!(ref self, count_ones)  }
    pub fn count_zeros(&self) -> u32   { delegate!(ref self, count_zeros) }

    pub fn contains(&self, bit: u16)   -> bool { delegate!(ref self, contains, bit)   }
    pub fn insert(&mut self, bit: u16) -> bool { delegate!(ref mut self, insert, bit) }
    pub fn remove(&mut self, bit: u16) -> bool { delegate!(ref mut self, remove, bit) }

    pub fn iter(&self) -> block::Iter {
        match *self {
            Seq16(ref data) => data.iter(),
            Seq64(ref data) => data.iter(),
            Rle16(ref data) => data.iter(),
        }
    }
}

impl ::std::ops::Index<u16> for Vec16 {
    type Output = bool;
    fn index(&self, i: u16) -> &Self::Output {
        if self.contains(i) {
            ::TRUE
        } else {
            ::FALSE
        }
    }
}

impl<'a> ::std::iter::IntoIterator for &'a Vec16 {
    type Item = <block::Iter<'a> as Iterator>::Item;
    type IntoIter = block::Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        match *self {
            Seq16(ref data) => data.iter(),
            Seq64(ref data) => data.iter(),
            Rle16(ref data) => data.iter(),
        }
    }
}

impl ::std::iter::Extend<u16> for Vec16 {
    fn extend<I>(&mut self, iterable: I)
    where
        I: ::std::iter::IntoIterator<Item = u16>,
    {
        extend_by_u16!(self, iterable);
    }
}

impl ::std::iter::FromIterator<u16> for Vec16 {
    fn from_iter<I>(iterable: I) -> Self
    where
        I: ::std::iter::IntoIterator<Item = u16>,
    {
        let iter = iterable.into_iter();
        let mut block = Vec16::new();
        let ones = extend_by_u16!(&mut block, iter);
        debug_assert_eq!(ones, block.count_ones());
        block
    }
}
impl<'a> ::std::iter::FromIterator<&'a u16> for Vec16 {
    fn from_iter<I>(iterable: I) -> Self
    where
        I: ::std::iter::IntoIterator<Item = &'a u16>,
    {
        let iter = iterable.into_iter();
        iter.cloned().collect::<Self>()
    }
}

impl ::std::iter::FromIterator<bool> for Vec16 {
    fn from_iter<I>(iterable: I) -> Self
    where
        I: ::std::iter::IntoIterator<Item = bool>,
    {
        let iter = iterable.into_iter();
        iter.take(Self::CAPACITY as usize)
            .enumerate()
            .filter_map(|(i, p)| if p { Some(i as u16) } else { None })
            .collect::<Self>()
    }
}
impl<'a> ::std::iter::FromIterator<&'a bool> for Vec16 {
    fn from_iter<I>(iterable: I) -> Self
    where
        I: ::std::iter::IntoIterator<Item = &'a bool>,
    {
        let iter = iterable.into_iter();
        iter.cloned().collect::<Self>()
    }
}
