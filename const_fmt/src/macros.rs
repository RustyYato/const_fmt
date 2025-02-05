macro_rules! tri {
    ($value:expr) => {
        match $value {
            Ok(x) => x,
            Err(x) => return Err(x),
        }
    };
}
