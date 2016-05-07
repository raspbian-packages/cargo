// Copyright 2013-2015 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Formatting utilities for URLs.
//!
//! These formatters can be used to coerce various URL parts into strings.
//!
//! You can use `<formatter>.to_string()`, as the formatters implement `fmt::Display`.

use std::fmt::{self, Formatter};
use super::Url;

/// Formatter and serializer for URL path data.
pub struct PathFormatter<'a, T:'a> {
    /// The path as a slice of string-like objects (String or &str).
    pub path: &'a [T]
}

impl<'a, T: fmt::Display> fmt::Display for PathFormatter<'a, T> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if self.path.is_empty() {
            formatter.write_str("/")
        } else {
            for path_part in self.path {
                try!("/".fmt(formatter));
                try!(path_part.fmt(formatter));
            }
            Ok(())
        }
    }
}


/// Formatter and serializer for URL username and password data.
pub struct UserInfoFormatter<'a> {
    /// URL username as a string slice.
    pub username: &'a str,

    /// URL password as an optional string slice.
    ///
    /// You can convert an `Option<String>` with `.as_ref().map(|s| s)`.
    pub password: Option<&'a str>
}

impl<'a> fmt::Display for UserInfoFormatter<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if !self.username.is_empty() || self.password.is_some() {
            try!(formatter.write_str(self.username));
            if let Some(password) = self.password {
                try!(formatter.write_str(":"));
                try!(formatter.write_str(password));
            }
            try!(formatter.write_str("@"));
        }
        Ok(())
    }
}


/// Formatter for URLs which ignores the fragment field.
pub struct UrlNoFragmentFormatter<'a> {
    pub url: &'a Url
}

impl<'a> fmt::Display for UrlNoFragmentFormatter<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        try!(formatter.write_str(&self.url.scheme));
        try!(formatter.write_str(":"));
        try!(self.url.scheme_data.fmt(formatter));
        if let Some(ref query) = self.url.query {
            try!(formatter.write_str("?"));
            try!(formatter.write_str(query));
        }
        Ok(())
    }
}