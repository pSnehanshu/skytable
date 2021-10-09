/*
 * Created on Sat Oct 09 2021
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

//! This module provides a simple way to avoid "the funk" with "funky input queries". It simply
//! tokenizes char-by-char analyzing quotes et al as required
//!

use core::fmt;
use skytable::{types::RawString, Query};

#[derive(Debug)]
pub enum TokenizerError {
    QuoteMismatch(String),
    BadExpression(String),
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QuoteMismatch(expr) => write!(f, "mismatched quotes near end of: `{}`", expr),
            Self::BadExpression(expr) => write!(f, "bad expression near end of: `{}`", expr),
        }
    }
}

pub trait SequentialQuery {
    fn push(&mut self, input: &[u8]);
    fn new() -> Self;
}

#[cfg(test)]
impl SequentialQuery for Vec<String> {
    fn push(&mut self, input: &[u8]) {
        Vec::push(self, String::from_utf8_lossy(input).to_string())
    }
    fn new() -> Self {
        Vec::new()
    }
}

impl SequentialQuery for Query {
    fn push(&mut self, input: &[u8]) {
        Query::push(self, RawString::from(input.to_owned()))
    }
    fn new() -> Self {
        Query::new()
    }
}

pub fn get_query<T: SequentialQuery>(inp: &[u8]) -> Result<T, TokenizerError> {
    if inp.is_empty() {
        panic!("Input is empty");
    }
    let mut query = T::new();
    let mut it = inp.iter().peekable();
    macro_rules! pos {
        () => {
            inp.len() - it.len()
        };
    }
    while let Some(tok) = it.next() {
        match tok {
            // numbers? nope
            b'0'..=b'9' => {
                return Err(TokenizerError::BadExpression(
                    String::from_utf8_lossy(&inp[..pos!()]).to_string(),
                ));
            }
            b'\'' => {
                // hmm, quotes; let's see where it ends
                let pos = pos!();
                let qidx = it.position(|x| *x == b'\'');
                match qidx {
                    Some(idx) => query.push(&inp[pos..idx + pos]),
                    None => {
                        let end = pos!();
                        return Err(TokenizerError::QuoteMismatch(
                            String::from_utf8_lossy(&inp[pos..end]).to_string(),
                        ));
                    }
                }
            }
            b'"' => {
                // hmm, quotes; let's see where it ends
                let pos = pos!();
                let qidx = it.position(|x| *x == b'"');
                match qidx {
                    Some(idx) => query.push(&inp[pos..idx + pos]),
                    None => {
                        let end = pos!();
                        return Err(TokenizerError::QuoteMismatch(
                            String::from_utf8_lossy(&inp[pos..end]).to_string(),
                        ));
                    }
                }
            }
            b' ' => {
                // this just prevents control from being handed to the wildcard
                continue;
            }
            _ => {
                let start = pos!() - 1;
                let mut end = start;
                // alpha? cool, go on
                while let Some(tok) = it.peek() {
                    if **tok == b' ' {
                        it.next();
                        break;
                    } else {
                        end += 1;
                        it.next();
                    }
                }
                end += 1;
                query.push(&inp[start..end]);
            }
        }
    }
    Ok(query)
}