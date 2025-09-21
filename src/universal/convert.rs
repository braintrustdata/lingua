pub trait TryFromLLM<T>: Sized {
    type Error;

    // Required method
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

// Blanket implementation for Vec<T> -> Vec<U> conversions where U: TryConvert<T>
impl<T, U> TryFromLLM<Vec<T>> for Vec<U>
where
    U: TryFrom<T>,
{
    type Error = U::Error;

    fn try_from(values: Vec<T>) -> Result<Self, Self::Error> {
        values
            .into_iter()
            .map(|item| U::try_convert(item))
            .collect()
    }
}

pub trait TryIntoLLM<T>: Sized {
    type Error;

    // Required method
    fn try_into(self) -> Result<T, Self::Error>;
}

impl<T, U> TryIntoLLM<Vec<U>> for Vec<T>
where
    U: TryFrom<T>,
{
    type Error = U::Error;

    fn try_into(self) -> Result<Vec<U>, Self::Error> {
        self.into_iter().map(|item| U::try_from(item)).collect()
    }
}
