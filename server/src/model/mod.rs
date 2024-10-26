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

pub mod app_data;
pub mod author;
pub mod dimensions;
pub mod mime;
pub mod moose;
pub mod pages;
pub mod queries;
pub mod votes;

// constants
pub const PAGE_SIZE: usize = 12;
pub const PAGE_SEARCH_LIM: usize = 10;
// this is for PNG output, technically the line output is variable based on font x-height
pub const PIX_FMT_WIDTH: usize = 16;
pub const PIX_FMT_HEIGHT: usize = 24;
