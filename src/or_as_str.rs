pub trait OrAsStr<'a, 'b: 'a> {
    fn or_as_str(self, default: &'b str) -> &'a str;
    fn or_as_str_else(self, f: impl FnOnce() -> &'b str) -> &'a str;
}
macro_rules! impl_or_as_str_for_string {
    ($strtype:ty) => {
        impl<'a, 'b: 'a> OrAsStr<'a, 'b> for $strtype {
            fn or_as_str(self, default: &'b str) -> &'a str {
                match self {
                    Some(inner) => inner.as_str(),
                    None => default,
                }
            }
            fn or_as_str_else(self, f: impl FnOnce() -> &'b str) -> &'a str {
                match self {
                    Some(inner) => inner.as_str(),
                    None => f(),
                }
            }
        }
    };
}
impl_or_as_str_for_string!(&'a Option<String>);
impl_or_as_str_for_string!(Option<&'a String>);

impl<'a, 'b: 'a> OrAsStr<'a, 'b> for Option<&'a str> {
    fn or_as_str(self, default: &'b str) -> &'a str {
        match self {
            Some(inner) => inner,
            None => default,
        }
    }
    fn or_as_str_else(self, f: impl FnOnce() -> &'b str) -> &'a str {
        match self {
            Some(inner) => inner,
            None => f(),
        }
    }
}
