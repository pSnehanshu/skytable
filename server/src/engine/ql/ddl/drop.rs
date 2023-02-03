/*
 * Created on Wed Feb 01 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
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

use crate::engine::ql::{
    ast::{Entity, QueryData, State, Statement},
    lex::{Slice, Token},
    LangError, LangResult,
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
    pub const fn new(space: Slice<'a>, force: bool) -> Self {
        Self { space, force }
    }
    fn parse<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<DropSpace<'a>> {
        if state.cursor_is_ident() {
            let ident = state.fw_read();
            // should we force drop?
            let force = state.cursor_rounded_eq(Token::Ident(b"force"));
            state.cursor_ahead_if(force);
            // either `force` or nothing
            if state.exhausted() {
                return Ok(DropSpace::new(
                    unsafe {
                        // UNSAFE(@ohsayan): Safe because the match predicate ensures that tok[1] is indeed an ident
                        extract!(ident, Token::Ident(ref space) => *space)
                    },
                    force,
                ));
            }
        }
        Err(LangError::UnexpectedToken)
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
    fn parse<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<Self> {
        let e = Entity::attempt_process_entity_result(state)?;
        let force = state.cursor_rounded_eq(Token::Ident(b"force"));
        state.cursor_ahead_if(force);
        if state.exhausted() {
            return Ok(DropModel::new(e, force));
        } else {
            Err(LangError::UnexpectedToken)
        }
    }
}

// drop (<space> | <model>) <ident> [<force>]
/// ## Panic
///
/// If token stream length is < 2
pub fn parse_drop<'a, Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<Statement<'a>> {
    match state.fw_read() {
        Token![model] => DropModel::parse(state).map(Statement::DropModel),
        Token![space] => return DropSpace::parse(state).map(Statement::DropSpace),
        _ => Err(LangError::UnexpectedToken),
    }
}

pub use impls::DropStatementAST;
mod impls {
    use super::{DropModel, DropSpace};
    use crate::engine::ql::{
        ast::{traits::ASTNode, QueryData, State, Statement},
        LangResult,
    };
    impl<'a> ASTNode<'a> for DropModel<'a> {
        fn _from_state<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<Self> {
            Self::parse(state)
        }
    }
    impl<'a> ASTNode<'a> for DropSpace<'a> {
        fn _from_state<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<Self> {
            Self::parse(state)
        }
    }
    #[derive(sky_macros::Wrapper, Debug)]
    pub struct DropStatementAST<'a>(Statement<'a>);
    impl<'a> ASTNode<'a> for DropStatementAST<'a> {
        fn _from_state<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> LangResult<Self> {
            super::parse_drop(state).map(Self)
        }
    }
}
