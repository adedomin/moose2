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
