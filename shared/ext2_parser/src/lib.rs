#![no_std]

use enum_bitflags::bitor_flags;

const SUPER_BLOCK_OFFSET: usize = 1024;
const SUPER_BLOCK_SIZE: usize = 1024;
const SUPER_BLOCK_MAGIC_SIGNATURE: u16 = 0xEF53;
const INODE_DIRECT_PTR_COUNT: usize = 12;
pub const ROOT_INODE: u32 = 2;

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BlockAddr(u32);

enum OptionalFeatureFlags { 
    NewDirectoryPrealloc = 0x1,
    AFSServerInodes = 0x2,
    FileSystemJournal = 0x4,
    ExtendedInodeAttributes = 0x8,
    ResizableFilesystem = 0x10,
    HashedDirectoryIndex = 0x20,
}
bitor_flags!(OptionalFeatureFlags, u32);

enum RequiredFeatureFlags {
    CompressionUsed = 0x1,
    DirectoryEntriesContainTypeField = 0x2,
    JournalReplaying = 0x4,
    JournalDevice = 0x8,
}
bitor_flags!(RequiredFeatureFlags, u32);

enum WritingFeatureFlags { 
    SparseSuperblocksAndGroupDescriptorTables = 0x1,
    FileSize64Bit = 0x2,
    DirectoryContentsBinarySearchTree = 0x4,
}
bitor_flags!(WritingFeatureFlags, u32);

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct SuperBlock {
    inode_count: u32,
    block_count: u32,
    num_blocks_reserved_for_su: u32,
    unallocated_blocks_count: u32,
    unallocated_inodes_count: u32,
    superblock_block_number: BlockAddr,
    block_size_exponent: u32, /// log_2(block_size) - 10
    fragment_size_exponent: i32, /// log_2(frament_size) - 10
    num_blocks_in_block_group: u32,
    num_fragments_in_block_group: u32,
    num_inodes_in_block_group: u32,
    last_mount_time: u32,
    last_write_time: u32,
    mount_count_since_consistency_check: u16,
    num_mounts_before_consistency_check: u16,
    magic_signature: u16,
    filesystem_state: u16,
    error_handling_method: u16,
    minor_version: u16,
    last_consistency_check_time: u32,
    consistency_check_time_interval: u32,
    creator_operating_system_id: u32,
    major_version: u32,
    su_user_id: u16,
    su_group_id: u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct SuperBlockExtension {
    first_non_reserved_inode: u32,
    inode_size: u16,
    containing_block_group: u16,
    optional_feature_flags: u32,
    required_feature_flags: u32,
    writing_feature_flags: u32,
    filesystem_id: [u8; 16],
    volume_name_cstr: [u8; 16],
    last_mount_path_cstr: [u8; 64],
    compression_algorithm: u32, 
    file_block_preallocation_count: u8,
    directory_block_preallocation_count: u8,
    _unused: u16,
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_device: u32,
    orphan_inode_list_head: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct BlockGroupDescriptor {
    block_usage_bitmap_addr: BlockAddr,
    inode_usage_bitmap_addr: BlockAddr,
    inode_table_start_addr: BlockAddr,
    unallocated_blocks_count: u16,
    unallocated_inodes_count: u16,
    directories_count: u16,
    _unused: [u8; 14],
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Inode {
    pub type_and_perms: u16, // Low 12 bits are perms, upper 4 bits are type
    pub user_id: u16,
    pub size_low: u32,
    pub last_access_time: u32,
    pub creation_time: u32,
    pub last_modification_time: u32,
    pub deletion_time: u32,
    pub group_id: u16,
    pub hard_link_count: u16,   // How many hard links point to this inode. When it reaches 0, data blocks
                                // should be marked unallocated
    pub disk_sector_count: u32, // Disk sectors (512 bytes) in use (not counting the inode structure)
    pub flags: u32,
    pub os_specific_1: u32,
    pub direct_pointers: [BlockAddr; INODE_DIRECT_PTR_COUNT],
    pub singly_indirect_pointer: BlockAddr,
    pub doubly_indirect_pointer: BlockAddr,
    pub triply_indirect_pointer: BlockAddr,
    pub generation_number: u32,
    pub extended_attributes_block: BlockAddr,
    pub size_high: u32,
    pub fragment_block: BlockAddr,
    pub os_specific_2: [u8; 12],
}

enum InodeType {
    FIFO = 0x1,
    CharacterDevice = 0x2,
    Directory = 0x4,
    BlockDevice = 0x6,
    RegularFile = 0x8,
    SymbolicLink = 0xA,
    UnixSocket = 0xC,
}

enum InodePermissions {
    OtherExecute = 0o1,
    OtherWrite = 0o2,
    OtherRead = 0o4,
    GroupExecute = 0o10,
    GroupWrite = 0o20,
    GroupRead = 0o40,
    UserExecute = 0o100,
    UserWrite = 0o200,
    UserRead = 0o400,
    StickyBit = 0o1000,
    SetGroupID = 0o2000,
    SetUserID = 0o4000,
}
bitor_flags!(InodePermissions, u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DirEntryType {
    Unknown = 0,
    RegularFile = 1,
    Directory = 2,
    CharacterDevice = 3,
    BlockDevice = 4,
    Buffer = 5,
    Socket = 6,
    SymbolicLink = 7,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct DirectoryEntry {
    inode: u32,
    size: u16,
    name_length: u8,
    type_indicator: DirEntryType,
}

#[derive(Debug)]
pub struct Ext2Parser<'a> {
    raw_bytes: &'a [u8],
    super_block: &'a SuperBlock,
    super_block_extension: &'a SuperBlockExtension,
    block_group_descriptor_table: &'a [BlockGroupDescriptor],

    block_size: usize,
    inode_count: u32,
    block_count: u32,
    blocks_per_block_group: u32,
    inodes_per_block_group: u32,
    block_group_count: u32,
    num_ptrs_per_block: usize,
}

#[derive(PartialEq, Eq)]
pub enum IterationDecision {
    Continue,
    Break,
}

impl<'a> Ext2Parser<'a> {
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {

        if bytes.len() < SUPER_BLOCK_OFFSET + SUPER_BLOCK_SIZE {
            return None;
        }

        let super_block = unsafe { &*(bytes[SUPER_BLOCK_OFFSET..].as_ptr() as *const SuperBlock) };
        if super_block.magic_signature != SUPER_BLOCK_MAGIC_SIGNATURE {
            return None;
        }

        if super_block.major_version < 1 {
            return None;
        }

        let extended_fields_offset = SUPER_BLOCK_SIZE + core::mem::size_of::<SuperBlock>();
        let super_block_extension = unsafe {
            &*(bytes[extended_fields_offset..].as_ptr() as *const SuperBlockExtension)
        };

        if super_block_extension.inode_size != core::mem::size_of::<Inode>() as u16 {
            return None;
        }

        let supported_features_mask = RequiredFeatureFlags::DirectoryEntriesContainTypeField as u32;
        if (super_block_extension.required_feature_flags & !supported_features_mask) != 0 {
            // Unsupported required feature
            return None;
        }

        // TODO: Actually support
        let supported_writing_features_mask = WritingFeatureFlags::SparseSuperblocksAndGroupDescriptorTables
            | WritingFeatureFlags::FileSize64Bit;
        if (super_block_extension.writing_feature_flags & !supported_writing_features_mask) != 0 {
            // Unsupported writing feature TODO: Read-only mode
            return None;
        }

        let block_size = 1024usize.checked_shl(super_block.block_size_exponent)?;

        let block_group_count = 
            div_ceil(super_block.block_count, super_block.num_blocks_in_block_group)?;
        let block_group_count_alt = 
            div_ceil(super_block.inode_count, super_block.num_blocks_in_block_group)?;
        if block_group_count != block_group_count_alt {
            return None;
        }

        if bytes.len() < block_size.checked_mul(super_block.block_count as usize)? {
            return None;
        }

        let block_group_descriptor_table_offset = (super_block.superblock_block_number.0 as usize + 1) * block_size;
        let block_group_descriptor_table = unsafe { core::slice::from_raw_parts(
                bytes[block_group_descriptor_table_offset..].as_ptr() as *const BlockGroupDescriptor,
                block_group_count as usize
        )};

        Some(Self {
            raw_bytes: bytes,
            super_block,
            super_block_extension,
            block_group_descriptor_table,
            block_size,
            inode_count: super_block.inode_count,
            block_count: super_block.block_count,
            blocks_per_block_group: super_block.num_blocks_in_block_group,
            inodes_per_block_group: super_block.num_inodes_in_block_group,
            block_group_count,
            num_ptrs_per_block: block_size / core::mem::size_of::<BlockAddr>()
        })
    }

    /// (next_opaque_offset, inode, filename, entry_type)
    pub fn get_next_directory_entry(&self, inode: u32, opaque_offset: u32)
        -> Option<(u32, u32, &'a str, DirEntryType)> {
        // FIXME: Don't iterate from the start everytime
        let mut total_offset: u32 = 0;
        let mut result = None;
        self.for_each_data_block(inode, &mut |data_block| {
            let mut curr_offset = 0;
            while curr_offset < self.block_size {
                let dir_entry = unsafe {
                    &*(data_block[curr_offset..].as_ptr() as *const DirectoryEntry)
                };

                if dir_entry.size == 0 {
                    return IterationDecision::Break;
                }

                if dir_entry.inode != 0 {
                    if total_offset == opaque_offset {
                        let filename_offset = curr_offset + core::mem::size_of::<DirectoryEntry>();
                        let filename = core::str::from_utf8(
                            &data_block[filename_offset..filename_offset+dir_entry.name_length as usize]
                        ).unwrap();

                        result = Some((
                            total_offset + dir_entry.size as u32,
                            dir_entry.inode,
                            filename,
                            dir_entry.type_indicator
                        ));
                        return IterationDecision::Break;
                    } else if total_offset > opaque_offset {
                        return IterationDecision::Break;
                    }
                }

                curr_offset += dir_entry.size as usize;
                total_offset += dir_entry.size as u32;
            }

            IterationDecision::Continue
        });

        result
    }

    pub fn for_each_directory_entry<F>(&self, inode: u32, mut callback: F)
        where F: FnMut(u32, &'a str, DirEntryType) -> IterationDecision {
        self.for_each_data_block(inode, &mut |data_block| {
            let mut curr_offset = 0;
            while curr_offset < self.block_size {
                let dir_entry = unsafe {
                    &*(data_block[curr_offset..].as_ptr() as *const DirectoryEntry)
                };

                if dir_entry.size == 0 {
                    return IterationDecision::Break;
                }

                if dir_entry.inode != 0 {
                    let filename_offset = curr_offset + core::mem::size_of::<DirectoryEntry>();
                    let filename = core::str::from_utf8(
                        &data_block[filename_offset..filename_offset+dir_entry.name_length as usize]
                    ).unwrap();

                    if callback(dir_entry.inode, filename, dir_entry.type_indicator) == IterationDecision::Break {
                        return IterationDecision::Break;
                    }
                }

                curr_offset += dir_entry.size as usize;
            }

            IterationDecision::Continue
        });
    }

    pub fn resolve_path_to_inode(&self, path: &str, mut base_inode: u32) -> Option<(u32, DirEntryType)> {
        if path == "/" {
            return Some((ROOT_INODE, DirEntryType::Directory));
        }

        let path = if path.starts_with("/") {
            base_inode = ROOT_INODE;
            &path[1..]
        } else {
            path
        };

        let path = if path.ends_with("/") {
            &path[..path.len()-1]
        } else {
            path
        };
        
        let mut inode = base_inode;
        let mut entry_type = DirEntryType::Directory;
        let mut reached_file = false;

        for component in path.split('/') {
            if component == "" || reached_file {
                return None;
            }

            let mut found_match = false;
            self.for_each_directory_entry(inode, |child_inode, child_name, child_type| {
                if child_name == component {
                    inode = child_inode;
                    entry_type = child_type;

                    match child_type {
                        DirEntryType::BlockDevice | DirEntryType::Buffer
                        | DirEntryType::CharacterDevice | DirEntryType::RegularFile
                        | DirEntryType::Socket | DirEntryType::Unknown => {
                            reached_file = true;
                        }
                        DirEntryType::Directory => {}
                        DirEntryType::SymbolicLink => todo!("Handle symbolic links")
                    }

                    found_match = true;
                    return IterationDecision::Break;
                }

                IterationDecision::Continue
            });

            if !found_match {
                return None;
            }
        }

        Some((inode, entry_type))
    }

    pub fn get_contents(&self, inode: u32, out_buffer: &mut [u8]) -> usize {
        self.get_contents_with_offset(inode, out_buffer, 0)
    }

    pub fn get_contents_with_offset(&self, inode: u32, out_buffer: &mut [u8], offset: usize) -> usize {
        if out_buffer.len() == 0 {
            return 0;
        }

        // FIXME: Don't iterate from the start every time...

        let inode_metadata = self.get_inode(inode);
        let file_size = inode_metadata.size_low as usize; // TODO: 64bit size

        let mut total_read = 0;
        let mut data_offset = 0;
        self.for_each_data_block(inode, &mut |data_block| {
            let block_length = data_block.len().min(file_size - data_offset);

            if offset < data_offset + block_length {
                let block_offset = if offset > data_offset {
                    offset - data_offset
                } else {
                    0
                };

                let left_in_block = block_length - block_offset;
                let size_left = left_in_block.min(out_buffer.len() - total_read);
                out_buffer[total_read..total_read+size_left].copy_from_slice(&data_block[..size_left]);
                total_read += size_left;

                if total_read == out_buffer.len() {
                    return IterationDecision::Break;
                }
            }

            data_offset += self.block_size;
            if data_offset >= file_size {
                IterationDecision::Break
            } else {
                IterationDecision::Continue
            }
        });

        total_read
    }

    pub fn for_each_data_block<F>(&self, inode: u32, callback: &mut F)
        where F: FnMut(&'a [u8]) -> IterationDecision {
        let inode_metadata = self.get_inode(inode);

        for i in 0..INODE_DIRECT_PTR_COUNT {
            if callback(self.get_block(inode_metadata.direct_pointers[i])) == IterationDecision::Break {
                return;
            }
        }

        self.for_each_indirect_block(inode_metadata.singly_indirect_pointer, callback);
        self.for_each_doubly_indirect_block(inode_metadata.doubly_indirect_pointer, callback);
        self.for_each_triply_indirect_block(inode_metadata.triply_indirect_pointer, callback);
    }

    pub fn get_inode(&self, inode: u32) -> &'a Inode {
        assert!(inode >= 1);
        assert!(inode <= self.inode_count);

        let block_group = ((inode - 1) / self.inodes_per_block_group) as usize;
        let inode_index = ((inode - 1) % self.inodes_per_block_group) as usize;
        let inode_table_block_addr = 
            self.block_group_descriptor_table[block_group].inode_table_start_addr.0 as usize;
        let inode_offset = 
            (inode_table_block_addr * self.block_size) + (inode_index * core::mem::size_of::<Inode>());
        
        unsafe { 
            &*(self.raw_bytes[inode_offset..].as_ptr() as *const Inode)
        }
    }

    fn get_block(&self, block: BlockAddr) -> &'a [u8] {
        let offset = block.0 as usize * self.block_size;
        &self.raw_bytes[offset..offset+self.block_size]
    }

    fn get_ptrs_block(&self, block: BlockAddr) -> &'a [BlockAddr] {
        let ptrs_block_offset = block.0 as usize * self.block_size;
        unsafe { 
            core::slice::from_raw_parts(
                self.raw_bytes.as_ptr().add(ptrs_block_offset) as *const BlockAddr,
                self.num_ptrs_per_block
            )
        }
    }

    fn for_each_indirect_block<F>(&self, block: BlockAddr, callback: &mut F)
        where F: FnMut(&'a [u8]) -> IterationDecision {
        let ptrs = self.get_ptrs_block(block);
        for &direct_ptr in ptrs {
            if direct_ptr.0 == 0 {
                return;
            }

            if callback(self.get_block(direct_ptr)) == IterationDecision::Break {
                return;
            }
        }
    }

    fn for_each_doubly_indirect_block<F>(&self, block: BlockAddr, callback: &mut F)
        where F: FnMut(&'a [u8]) -> IterationDecision {
        let ptrs = self.get_ptrs_block(block);

        for &ptr in ptrs {
            if ptr.0 == 0 {
                return;
            }
            self.for_each_indirect_block(ptr, callback);
        }
    }

    fn for_each_triply_indirect_block<F>(&self, block: BlockAddr, callback: &mut F)
        where F: FnMut(&'a [u8]) -> IterationDecision {
        let ptrs = self.get_ptrs_block(block);

        for &ptr in ptrs {
            if ptr.0 == 0 {
                return;
            }
            self.for_each_doubly_indirect_block(ptr, callback);
        }
    }
}

fn div_ceil(x: u32, y: u32) -> Option<u32> {
    if y == 0 {
        return None;
    }

    if x == 0 {
        return Some(0);
    }

    Some(1 + ((x - 1) / y))
}

#[cfg(test)]
mod tests {

    use crate::*;
    extern crate std;

    #[test]
    fn it_works() {
        let file = std::fs::read("test_ext2_1024.fs").unwrap();
        let parser = Ext2Parser::parse(&file).unwrap();

        parser.for_each_directory_entry(2, |inode, name, entry_type| {
            std::println!("{:#?} {:#?} {:#?}", inode, name, entry_type);
            IterationDecision::Continue
        });
        let mut buffer = [0u8; 4096];
        let length = parser.get_contents(15, &mut buffer);
        let contents = &buffer[..length];
        std::println!("{:?}", contents);
        parser.for_each_directory_entry(12, |inode, name, entry_type| {
            std::println!("{:#?} {:#?} {:#?}", inode, name, entry_type);
            IterationDecision::Continue
        });

        panic!();

        std::println!("{:#?}", parser);
        panic!();
    }
}
