#![doc = include_str!("../README.md")]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

mod ascii;
mod bash;
mod sh;

pub use bash::Bash;
pub use sh::Sh;

/// Extension trait for pushing shell quoted byte slices, e.g. `&[u8]`, [`&str`]
/// – anything that's [`Quotable`] – into byte container types like [`Vec<u8>`],
/// [`String`], [`OsString`] on Unix, and [`bstr::BString`] if it's enabled
pub trait QuoteExt {
    fn push_quoted<'a, Q: Quoter, S: ?Sized + Into<Quotable<'a>>>(&mut self, q: Q, s: S);
}

impl QuoteExt for Vec<u8> {
    fn push_quoted<'a, Q: Quoter, S: ?Sized + Into<Quotable<'a>>>(&mut self, _q: Q, s: S) {
        Q::quote_into(s, self);
    }
}

#[cfg(unix)]
impl QuoteExt for OsString {
    fn push_quoted<'a, Q: Quoter, S: ?Sized + Into<Quotable<'a>>>(&mut self, _q: Q, s: S) {
        use std::os::unix::ffi::OsStrExt;
        let quoted = Q::quote(s);
        self.push(OsStr::from_bytes(&quoted))
    }
}

#[cfg(feature = "bstr")]
impl QuoteExt for bstr::BString {
    fn push_quoted<'a, Q: Quoter, S: ?Sized + Into<Quotable<'a>>>(&mut self, _q: Q, s: S) {
        Q::quote_into(s, self)
    }
}

impl QuoteExt for String {
    fn push_quoted<'a, Q: Quoter, S: ?Sized + Into<Quotable<'a>>>(&mut self, _q: Q, s: S) {
        let quoted = Q::quote(s);
        // SAFETY: `quoted` is valid UTF-8 (ASCII, in truth) because it was
        // generated by a `quote` implementation from this crate –
        // enforced because `Quoter` is sealed.
        let quoted = unsafe { std::str::from_utf8_unchecked(&quoted) };
        self.push_str(quoted);
    }
}

// ----------------------------------------------------------------------------

/// Extension trait for shell quoting many different owned and reference types,
/// e.g. `&[u8]`, [`&str`] – anything that's [`Quotable`] – into owned types
/// like [`Vec<u8>`], [`String`], [`OsString`] on Unix, and [`bstr::BString`] if
/// it's enabled.
pub trait QuoteRefExt<Output> {
    fn quoted<Q: Quoter>(self, q: Q) -> Output;
}

impl<'a, S> QuoteRefExt<Vec<u8>> for S
where
    S: ?Sized + Into<Quotable<'a>>,
{
    fn quoted<Q: Quoter>(self, _q: Q) -> Vec<u8> {
        Q::quote(self)
    }
}

#[cfg(unix)]
impl<'a, S> QuoteRefExt<OsString> for S
where
    S: ?Sized + Into<Quotable<'a>>,
{
    fn quoted<Q: Quoter>(self, _q: Q) -> OsString {
        use std::os::unix::ffi::OsStringExt;
        let quoted = Q::quote(self);
        OsString::from_vec(quoted)
    }
}

#[cfg(feature = "bstr")]
impl<'a, S> QuoteRefExt<bstr::BString> for S
where
    S: ?Sized + Into<Quotable<'a>>,
{
    fn quoted<Q: Quoter>(self, _q: Q) -> bstr::BString {
        let quoted = Q::quote(self);
        bstr::BString::from(quoted)
    }
}

impl<'a, S> QuoteRefExt<String> for S
where
    S: ?Sized + Into<Quotable<'a>>,
{
    fn quoted<Q: Quoter>(self, _q: Q) -> String {
        let quoted = Q::quote(self);
        // SAFETY: `quoted` is valid UTF-8 (ASCII, in truth) because it was
        // generated by a `quote` implementation from this crate –
        // enforced because `Quoter` is sealed.
        unsafe { String::from_utf8_unchecked(quoted) }
    }
}

// ----------------------------------------------------------------------------

pub(crate) mod quoter {
    pub trait QuoterSealed {
        /// Quote/escape a string of bytes into a new [`Vec<u8>`].
        fn quote<'a, S: ?Sized + Into<super::Quotable<'a>>>(s: S) -> Vec<u8>;

        /// Quote/escape a string of bytes into an existing [`Vec<u8>`].
        fn quote_into<'a, S: ?Sized + Into<super::Quotable<'a>>>(s: S, sout: &mut Vec<u8>);
    }
}

/// A trait for quoting/escaping a string of bytes into a shell-safe form.
///
/// **This trait is sealed** and cannot be implemented outside of this crate.
/// This is because the [`QuoteExt`] implementation for [`String`] must be sure
/// that quoted bytes are valid UTF-8, and that is only possible if the
/// implementation is known and tested in advance.
pub trait Quoter: quoter::QuoterSealed {}

// ----------------------------------------------------------------------------

/// A string of bytes that can be quoted/escaped.
///
/// This is used by many methods in this crate as a generic `Into<Quotable>`
/// constraint. Why not accept [`AsRef<[u8]>`] instead? The ergonomics of that
/// approach were not so good. For example, quoting [`OsString`]/[`OsStr`] and
/// [`PathBuf`]/[`Path`] didn't work in a natural way.
pub struct Quotable<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> From<&'a [u8]> for Quotable<'a> {
    fn from(source: &'a [u8]) -> Quotable<'a> {
        Quotable { bytes: source }
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Quotable<'a> {
    fn from(source: &'a [u8; N]) -> Quotable<'a> {
        Quotable { bytes: &source[..] }
    }
}

impl<'a> From<&'a Vec<u8>> for Quotable<'a> {
    fn from(source: &'a Vec<u8>) -> Quotable<'a> {
        Quotable { bytes: source }
    }
}

impl<'a> From<&'a str> for Quotable<'a> {
    fn from(source: &'a str) -> Quotable<'a> {
        source.as_bytes().into()
    }
}

impl<'a> From<&'a String> for Quotable<'a> {
    fn from(source: &'a String) -> Quotable<'a> {
        source.as_bytes().into()
    }
}

#[cfg(unix)]
impl<'a> From<&'a OsStr> for Quotable<'a> {
    fn from(source: &'a OsStr) -> Quotable<'a> {
        use std::os::unix::ffi::OsStrExt;
        source.as_bytes().into()
    }
}

#[cfg(unix)]
impl<'a> From<&'a OsString> for Quotable<'a> {
    fn from(source: &'a OsString) -> Quotable<'a> {
        use std::os::unix::ffi::OsStrExt;
        source.as_bytes().into()
    }
}

#[cfg(feature = "bstr")]
impl<'a> From<&'a bstr::BStr> for Quotable<'a> {
    fn from(source: &'a bstr::BStr) -> Quotable<'a> {
        let bytes: &[u8] = source.as_ref();
        bytes.into()
    }
}

#[cfg(feature = "bstr")]
impl<'a> From<&'a bstr::BString> for Quotable<'a> {
    fn from(source: &'a bstr::BString) -> Quotable<'a> {
        let bytes: &[u8] = source.as_ref();
        bytes.into()
    }
}

#[cfg(unix)]
impl<'a> From<&'a Path> for Quotable<'a> {
    fn from(source: &'a Path) -> Quotable<'a> {
        source.as_os_str().into()
    }
}

#[cfg(unix)]
impl<'a> From<&'a PathBuf> for Quotable<'a> {
    fn from(source: &'a PathBuf) -> Quotable<'a> {
        source.as_os_str().into()
    }
}
