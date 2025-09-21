// This is basically the same thing as TryFrom, but defined in this crate, so we can implement it
// for types outside our crate (like Message).
pub trait TryConvert<T>: Sized {
    type Error;

    // Required method
    fn try_convert(value: T) -> Result<Self, Self::Error>;
}

// Blanket implementation for Vec<T> -> Vec<U> conversions where U: TryConvert<T>
impl<T, U> TryConvert<Vec<T>> for Vec<U>
where
    U: TryConvert<T>,
{
    type Error = U::Error;

    fn try_convert(values: Vec<T>) -> Result<Self, Self::Error> {
        values
            .into_iter()
            .map(|item| U::try_convert(item))
            .collect()
    }
}
