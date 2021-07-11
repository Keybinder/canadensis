//! A 48-bit unsigned integer type

use core::convert::{TryFrom, TryInto};
use core::num::TryFromIntError;
use core::ops::{Add, Div, Mul, Rem, Shr, Sub};

use crate::InvalidValue;

const U48_MASK: u64 = 0x0000_ffff_ffff_ffff;

/// A 48-bit unsigned integer
// Invariant: the 16 most significant bits are zero
#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct U48(u64);

impl U48 {
    /// The minimum 48-bit integer value
    pub const MIN: U48 = U48(0);
    /// The maximum 48-bit integer value
    pub const MAX: U48 = U48(U48_MASK);

    /// Adds two integers, wrapping on overflow
    pub fn wrapping_add(&self, rhs: Self) -> Self {
        let inner_sum = self.0.wrapping_add(rhs.0);
        U48(inner_sum & U48_MASK)
    }

    /// Subtracts two integers, wrapping on underflow
    pub fn wrapping_sub(&self, rhs: Self) -> Self {
        let inner_difference = self.0.wrapping_sub(rhs.0);
        U48(inner_difference & U48_MASK)
    }
}

impl From<u8> for U48 {
    fn from(small: u8) -> Self {
        U48(small.into())
    }
}
impl From<u16> for U48 {
    fn from(small: u16) -> Self {
        U48(small.into())
    }
}
impl From<u32> for U48 {
    fn from(small: u32) -> Self {
        U48(small.into())
    }
}
impl TryFrom<i8> for U48 {
    type Error = TryFromIntError;
    fn try_from(small: i8) -> Result<Self, Self::Error> {
        Ok(U48(small.try_into()?))
    }
}
impl TryFrom<i16> for U48 {
    type Error = TryFromIntError;
    fn try_from(small: i16) -> Result<Self, Self::Error> {
        Ok(U48(small.try_into()?))
    }
}
impl TryFrom<i32> for U48 {
    type Error = TryFromIntError;
    fn try_from(small: i32) -> Result<Self, Self::Error> {
        Ok(U48(small.try_into()?))
    }
}
impl TryFrom<u64> for U48 {
    type Error = InvalidValue;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if (value & !U48_MASK) == 0 {
            // Higher bits are clear
            Ok(U48(value))
        } else {
            Err(InvalidValue)
        }
    }
}

impl From<U48> for u64 {
    fn from(small: U48) -> Self {
        small.0
    }
}

impl TryFrom<U48> for u32 {
    type Error = InvalidValue;

    fn try_from(value: U48) -> Result<Self, Self::Error> {
        if (value.0 & !u64::from(u32::MAX)) == 0 {
            Ok(value.0 as u32)
        } else {
            Err(InvalidValue)
        }
    }
}

impl Add for U48 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let inner_sum = self.0 + rhs.0;
        // Like a primitive type, check for overflow and panic in debug mode, or wrap in release mode
        #[cfg(debug_assertions)]
        {
            if (inner_sum & !U48_MASK) != 0 {
                panic!("Attempted to add with overflow");
            } else {
                U48(inner_sum)
            }
        }
        #[cfg(not(debug_assertions))]
        {
            U48(inner_sum & U48_MASK)
        }
    }
}

impl Sub for U48 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let inner_difference = self.0 - rhs.0;
        // Like a primitive type, check for overflow and panic in debug mode, or wrap in release mode
        #[cfg(debug_assertions)]
        {
            if (inner_difference & !U48_MASK) != 0 {
                panic!("Attempted to subtract with overflow");
            } else {
                U48(inner_difference)
            }
        }
        #[cfg(not(debug_assertions))]
        {
            U48(inner_difference & U48_MASK)
        }
    }
}

impl Div<u32> for U48 {
    type Output = Self;

    fn div(self, rhs: u32) -> Self::Output {
        U48(self.0 / u64::from(rhs))
    }
}

impl Shr<u32> for U48 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        U48(self.0 >> rhs)
    }
}

impl Rem<u32> for U48 {
    type Output = Self;

    fn rem(self, rhs: u32) -> Self::Output {
        U48(self.0 % u64::from(rhs))
    }
}

impl Mul<u32> for U48 {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        let inner_product = self.0 * u64::from(rhs);
        // Like a primitive type, check for overflow and panic in debug mode, or wrap in release mode
        #[cfg(debug_assertions)]
        {
            if (inner_product & !U48_MASK) != 0 {
                panic!("Attempted to subtract with overflow");
            } else {
                U48(inner_product)
            }
        }
        #[cfg(not(debug_assertions))]
        {
            U48(inner_product & U48_MASK)
        }
    }
}

macro_rules! delegate_format {
    ($($trait_name:ident,)+) => {
        $(
            impl core::fmt::$trait_name for U48 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::$trait_name::fmt(&self.0, f)
                }
            }
        )+
    }
}

delegate_format! {
    Debug,
    Display,
    UpperHex,
    LowerHex,
}

#[cfg(test)]
mod test_u48 {
    use super::U48;
    use crate::time::{Instant, Microseconds48};
    use core::cmp::Ordering;
    use core::convert::TryFrom;

    fn u48(value: u64) -> U48 {
        U48::try_from(value).unwrap()
    }

    #[test]
    fn u48_wrapping_sub() {
        assert_eq!(u48(0).wrapping_sub(u48(1)), u48(0xffff_ffff_ffff));
        assert_eq!(u48(0).wrapping_sub(u48(0x10)), u48(0xffff_ffff_fff0));
    }

    #[test]
    fn instant_u48_compare() {
        fn compare(ticks1: u64, ticks2: u64) -> Ordering {
            let ticks1 = u48(ticks1);
            let ticks2 = u48(ticks2);
            Microseconds48::new(ticks1).overflow_safe_compare(&Microseconds48::new(ticks2))
        }

        // Basic equality
        assert_eq!(compare(0, 0), Ordering::Equal);
        assert_eq!(compare(127, 127), Ordering::Equal);
        assert_eq!(compare(255, 255), Ordering::Equal);
        assert_eq!(compare(0xffff_ffff_ffff, 0xffff_ffff_ffff), Ordering::Equal);

        // With a difference of less than or equal to 2^47 - 1, comparison assumes that overflow
        // hasn't happened and works normally
        assert_eq!(compare(0, 10), Ordering::Less);
        assert_eq!(compare(0, 0xff_fffe), Ordering::Less);
        assert_eq!(compare(0, 0xff_ffff), Ordering::Less);
        assert_eq!(compare(0, 0x7fff_ffff_fffe), Ordering::Less);
        assert_eq!(compare(0, 0x7fff_ffff_ffff), Ordering::Less);
        // When the difference reaches 2^47, comparison thinks that overflow has happened and the
        // result is reversed.
        assert_eq!(compare(0, 0x8000_0000_0000), Ordering::Greater);
        assert_eq!(compare(0, 0x8000_0000_0001), Ordering::Greater);
        assert_eq!(compare(0, 0xffff_ffff_ffff), Ordering::Greater);
    }

    #[test]
    fn duration_u48() {
        fn duration(from: u64, to: u64) -> u64 {
            Microseconds48::new(u48(to))
                .duration_since(&Microseconds48::new(u48(from)))
                .as_microseconds()
                .0
        }
        // Basics
        assert_eq!(duration(0, 0), 0);
        assert_eq!(duration(0, 1), 1);
        assert_eq!(duration(0, 0xffff_ffff_ffff), 0xffff_ffff_ffff);
        // Overflow
        assert_eq!(duration(0xffff_ffff_ffff, 0), 1);
        assert_eq!(duration(0xffff_ffff_ffff, 1), 2);
        assert_eq!(
            duration(0xffff_ffff_ffff, 0xffff_ffff_fffe),
            0xffff_ffff_ffff
        );
    }
}
