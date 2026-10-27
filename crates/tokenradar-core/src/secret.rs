//! A wrapper that keeps secrets out of logs and wipes them from memory on drop (ADR-006).

use std::fmt;

use zeroize::Zeroize;

/// Holds a secret such as an OAuth access token.
///
/// - `Debug` and `Display` print `***`, so `tracing` fields and `{:?}` never leak it.
/// - The value is zeroized when dropped.
/// - It is deliberately not `Clone`, `Serialize` or `Deserialize`: copies must be explicit.
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrows the secret. Keep the borrow as short as the request that needs it.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<T: Zeroize> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "sk-ant-oat01-not-a-real-token";

    #[test]
    fn formatting_never_reveals_the_secret() {
        let secret = Secret::new(TOKEN.to_owned());
        for rendered in [
            format!("{secret:?}"),
            format!("{secret:#?}"),
            format!("{secret}"),
            format!("{:?}", Some(&secret)),
        ] {
            assert!(!rendered.contains("sk-ant"), "leaked: {rendered}");
        }
        assert_eq!(format!("{secret:?}"), "***");
        assert_eq!(secret.expose(), TOKEN);
    }

    #[test]
    fn secrets_inside_structs_stay_hidden() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Request {
            url: &'static str,
            token: Secret<String>,
        }
        let req = Request {
            url: "https://api.anthropic.com",
            token: TOKEN.to_owned().into(),
        };
        let rendered = format!("{req:?}");
        assert!(!rendered.contains(TOKEN));
        assert!(rendered.contains("***"));
    }

    #[test]
    fn drop_zeroizes_the_value() {
        use std::cell::Cell;
        use std::rc::Rc;

        // Observing freed memory would need `unsafe`; a probe records the call instead.
        struct Probe(Rc<Cell<bool>>);
        impl Zeroize for Probe {
            fn zeroize(&mut self) {
                self.0.set(true);
            }
        }

        let wiped = Rc::new(Cell::new(false));
        let secret = Secret::new(Probe(Rc::clone(&wiped)));
        assert!(!wiped.get());
        drop(secret);
        assert!(wiped.get());
    }
}
