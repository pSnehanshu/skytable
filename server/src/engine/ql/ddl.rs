/*
 * Created on Wed Nov 16 2022
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2022, Sayan Nandan <ohsayan@outlook.com>
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

#[cfg(test)]
use super::ast::InplaceData;
use {
    super::{
        ast::{Entity, QueryData, State, Statement},
        lexer::{Slice, Token},
        LangError, LangResult,
    },
    crate::util::compiler,
};

#[derive(Debug, PartialEq)]
/// A generic representation of `drop` query
pub struct DropSpace<'a> {
    pub(super) space: Slice<'a>,
    pub(super) force: bool,
}

impl<'a> DropSpace<'a> {
    #[inline(always)]
    /// Instantiate
    pub(super) const fn new(space: Slice<'a>, force: bool) -> Self {
        Self { space, force }
    }
}

#[derive(Debug, PartialEq)]
pub struct DropModel<'a> {
    pub(super) entity: Entity<'a>,
    pub(super) force: bool,
}

impl<'a> DropModel<'a> {
    #[inline(always)]
    pub fn new(entity: Entity<'a>, force: bool) -> Self {
        Self { entity, force }
    }
}

// drop (<space> | <model>) <ident> [<force>]
/// ## Panic
///
/// If token stream length is < 2
pub(super) fn parse_drop<'a, Qd: QueryData<'a>>(
    state: &mut State<'a, Qd>,
) -> LangResult<Statement<'a>> {
    match state.fw_read() {
        Token![model] => {
            // we have a model. now parse entity and see if we should force deletion
            let e = Entity::attempt_process_entity_result(state)?;
            let force = state.cursor_rounded_eq(Token::Ident(b"force"));
            state.cursor_ahead_if(force);
            // if we've exhausted the stream, we're good to go (either `force`, or nothing)
            if state.exhausted() {
                return Ok(Statement::DropModel(DropModel::new(e, force)));
            }
        }
        Token![space] if state.cursor_is_ident() => {
            let ident = state.fw_read();
            // should we force drop?
            let force = state.cursor_rounded_eq(Token::Ident(b"force"));
            state.cursor_ahead_if(force);
            // either `force` or nothing
            if state.exhausted() {
                return Ok(Statement::DropSpace(DropSpace::new(
                    unsafe {
                        // UNSAFE(@ohsayan): Safe because the match predicate ensures that tok[1] is indeed an ident
                        extract!(ident, Token::Ident(ref space) => *space)
                    },
                    force,
                )));
            }
        }
        _ => {}
    }
    Err(LangError::UnexpectedToken)
}

#[cfg(test)]
pub(super) fn parse_drop_full<'a>(tok: &'a [Token]) -> LangResult<Statement<'a>> {
    let mut state = State::new(tok, InplaceData::new());
    let r = self::parse_drop(&mut state);
    assert_full_tt!(state);
    r
}

pub(super) fn parse_inspect<'a, Qd: QueryData<'a>>(
    state: &mut State<'a, Qd>,
) -> LangResult<Statement<'a>> {
    /*
        inpsect model <entity>
        inspect space <entity>
        inspect spaces

        min length -> (<model> | <space>) <model> = 2
    */

    if compiler::unlikely(state.remaining() < 1) {
        return compiler::cold_rerr(LangError::UnexpectedEndofStatement);
    }

    match state.fw_read() {
        Token![model] => Entity::attempt_process_entity_result(state).map(Statement::InspectModel),
        Token![space] if state.cursor_has_ident_rounded() => {
            Ok(Statement::InspectSpace(unsafe {
                // UNSAFE(@ohsayan): Safe because of the match predicate
                extract!(state.fw_read(), Token::Ident(ref space) => space)
            }))
        }
        Token::Ident(id) if id.eq_ignore_ascii_case(b"spaces") && state.exhausted() => {
            Ok(Statement::InspectSpaces)
        }
        _ => {
            state.cursor_back();
            Err(LangError::ExpectedStatement)
        }
    }
}

#[cfg(test)]
pub(super) fn parse_inspect_full<'a>(tok: &'a [Token]) -> LangResult<Statement<'a>> {
    let mut state = State::new(tok, InplaceData::new());
    let r = self::parse_inspect(&mut state);
    assert_full_tt!(state);
    r
}
