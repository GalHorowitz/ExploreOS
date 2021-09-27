//! Small library that provides a macro for implementing bitwise-or for bitflag enums
#![no_std]

#[macro_export]
macro_rules! bitor_flags {
    ( $x:ty, $y:ty ) => {
        impl const core::ops::BitOr<$x> for $x {
            type Output = $y;
        
            fn bitor(self, rhs: $x) -> Self::Output {
                (self as Self::Output) | (rhs as Self::Output)
            }
        }
    };
}

#[cfg(test)]
mod tests {

    enum SomeEnum {
        OptionA = 1,
        OptionB = 2,
    }
    bitor_flags!(SomeEnum, u32);

    #[test]
    fn it_works() {
        assert_eq!(SomeEnum::OptionA | SomeEnum::OptionB, 3);
    }
}
