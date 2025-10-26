/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use serde::Serialize;

use crate::model::votes::VoteFlag;

use super::moose::Moose;

#[derive(Debug, Serialize)]
pub struct MooseSearch {
    /// The actual Moose page this moose belongs to.
    pub page: usize,
    pub moose: Moose,
}

#[derive(Debug, Default, Serialize)]
pub struct MooseSearchPage {
    /// number of pages returned by query set (max: 10)
    pub pages: usize,
    pub result: Vec<MooseSearch>,
}

#[derive(Debug, Serialize)]
pub struct MoosePage {
    #[serde(flatten)]
    pub moose: Moose,
    pub voted: VoteFlag,
}
