use ext2_parser::Ext2Parser;
use lock_cell::LockCell;

use crate::RAM_EXT2_FS;

pub static EXT2_PARSER: LockCell<Option<Ext2Parser>> = LockCell::new(None);

pub fn init() {
	*EXT2_PARSER.lock() = Ext2Parser::parse(RAM_EXT2_FS);
}