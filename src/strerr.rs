pub trait Strerr<T> {
    fn strerr(self) -> Result<T, String>;
}

impl<T, U: ToString> Strerr<T> for Result<T, U> {
    fn strerr(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}
