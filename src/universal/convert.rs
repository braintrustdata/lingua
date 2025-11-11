pub trait TryFromLLM<T>: Sized {
    type Error;

    // Required method
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

// Blanket implementation for Vec<T> -> Vec<U> conversions where U: TryFromLLM<T>
impl<T, U> TryFromLLM<Vec<T>> for Vec<U>
where
    U: TryFromLLM<T>,
{
    type Error = U::Error;

    fn try_from(values: Vec<T>) -> Result<Self, Self::Error> {
        values.into_iter().map(|item| U::try_from(item)).collect()
    }
}
