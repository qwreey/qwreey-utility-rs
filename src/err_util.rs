pub trait ErrToString<T> {
    fn err_to_string(self) -> Result<T, String>;
}
impl<T, U: ToString> ErrToString<T> for Result<T, U> {
    fn err_to_string(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

pub trait HeadingError<T> {
    fn heading_error(self, head: impl ToString) -> Result<T, String>;
    fn heading_error_else<G: ToString>(self, head: fn() -> G) -> Result<T, String>;
}
impl<T> HeadingError<T> for Result<T, String> {
    fn heading_error(self, head: impl ToString) -> Result<T, String> {
        self.map_err(|e| format!("{}{}", head.to_string(), e))
    }
    fn heading_error_else<G: ToString>(self, head: fn() -> G) -> Result<T, String> {
        self.map_err(|e| format!("{}{}", head().to_string(), e))
    }
}
