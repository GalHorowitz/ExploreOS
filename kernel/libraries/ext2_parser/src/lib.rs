#![no_std]
#![feature(const_trait_impl)]

use enum_bitflags::bitor_flags;

/// The offset from the start of the disk where the super block is located
const SUPER_BLOCK_OFFSET: usize = 1024;
/// The size in bytes of the super block
const SUPER_BLOCK_SIZE: usize = 1024;
/// The value of a the signature field of a valid super block
const SUPER_BLOCK_MAGIC_SIGNATURE: u16 = 0xEF53;
/// The number of direct pointers in an inode
const INODE_DIRECT_PTR_COUNT: usize = 12;
/// The inode number of the root directory
pub const ROOT_INODE: u32 = 2;

/// Bitmask of required features the implementation supports
const SUPPORTED_REQUIRED_FEATURES_MASK: u32 = 
    RequiredFeatureFlags::DirectoryEntriesContainTypeField as u32;
/// Bitmask of features required for writing the implemention supports
const SUPPORTED_WRITING_FEATURES_MASK: u32 = 
    WritingFeatureFlags::SparseSuperblocksAndGroupDescriptorTables | WritingFeatureFlags::FileSize64Bit;

/// An address in disk, as a multiple of the block size
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BlockAddr(u32);

/// The structure of the super-block, containing all the metadata about the filesystem
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct SuperBlock {
    /// Total count of inodes (used and free) in the filesystem
    inode_count: u32,
    /// Total count of blocks (used, free and reserved) in the filesystem
    block_count: u32,
    /// Number of blocks that are reserved for usage by the super user
    num_blocks_reserved_for_su: u32,
    /// Number of blocks that are free, including reserved blocks
    unallocated_blocks_count: u32,
    /// Number of inodes that are free
    unallocated_inodes_count: u32,
    /// The block containing the superblock
    superblock_block_number: BlockAddr,
    /// Encoded block size: `log2(block_size) - 10`
    block_size_exponent: u32,
    /// Encoded fragment size: `log2(fragment_size) - 10`
    fragment_size_exponent: i32,
    /// Number of blocks per block group
    num_blocks_in_block_group: u32,
    /// Number of fragments per block group
    num_fragments_in_block_group: u32,
    /// Number of inodes per block group
    num_inodes_in_block_group: u32,
    /// Last time the filesystem was mounted, in UNIX time
    last_mount_time: u32,
    /// Last time the filesystem was written to, in UNIX time
    last_write_time: u32,
    /// Number of times the filesystem was mounted since the last time its state was checked
    mount_count_since_consistency_check: u16,
    /// Number of times the filesystem can be mounted before a consistency check should be performed
    num_mounts_before_consistency_check: u16,
    /// EXT2 filesystem signature
    magic_signature: u16,
    /// The mount state of the filesystem. If the state is `mounted` before mounting, the filesystem
    /// was not cleanly unmounted and may contain errors
    filesystem_state: u16,
    /// Value which indicates what action the driver should take if an error was detected
    error_handling_method: u16,
    /// Minor revision level of the filesystem
    minor_version: u16,
    /// Last time the filesystem was checked, in UNIX time
    last_consistency_check_time: u32,
    /// Maximum time interval allowed between filesystem checks, in UNIX time
    consistency_check_time_interval: u32,
    /// Identifier of the operating system that created the filesystem
    creator_operating_system_id: u32,
    /// Major revision level of the filesystem
    major_version: u32,
    /// User id of the super user, i.e. the owner of reserved blocks
    su_user_id: u16,
    /// Group id of the super user, i.e. the owner of reserved blocks
    su_group_id: u16,
}

/// Fields added to the super block in major revision 1
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct SuperBlockExtension {
    /// The inode number of the first non-reserved inode
    first_non_reserved_inode: u32,
    /// The size in bytes of the inode structure
    inode_size: u16,
    /// The block group number containing this super block (useful for super block backups)
    containing_block_group: u16,
    /// Bitmask of features that the driver is not required to support in order to read/write, see
    /// [`OptionalFeatureFlags`]
    optional_feature_flags: u32,
    /// Bitmask of features that the driver is required to support in order to read/write, see
    /// [`RequiredFeatureFlags`]
    required_feature_flags: u32,
    /// Bitmask of features that the driver is required to support in order to write, see
    /// [`WritingFeatureFlags`]
    writing_feature_flags: u32,
    /// 128bit value uniquely identifying the filesystem 
    filesystem_id: [u8; 16],
    /// Name of the volume, usually unused
    volume_name_cstr: [u8; 16],
    /// The last mount-point path, usually unused
    last_mount_path_cstr: [u8; 64],
    /// The type of compression algorithm used, if compression is used
    compression_algorithm: u32, 
    /// Number of blocks the driver should attempt to pre-allocate for new files
    file_block_preallocation_count: u8,
    /// Number of blocks the driver should attempt to pre-allcoate for new directories
    directory_block_preallocation_count: u8,
    _unused: u16,
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_device: u32,
    orphan_inode_list_head: u32,
}

/// Feature flags that are not required for reading or writing from a filesystem
enum OptionalFeatureFlags {
    NewDirectoryPrealloc = 0x1,
    AFSServerInodes = 0x2,
    FileSystemJournal = 0x4,
    ExtendedInodeAttributes = 0x8,
    ResizableFilesystem = 0x10,
    HashedDirectoryIndex = 0x20,
}
bitor_flags!(OptionalFeatureFlags, u32);

/// Feature flags that are required to support reading from a filesystem
enum RequiredFeatureFlags {
    CompressionUsed = 0x1,
    DirectoryEntriesContainTypeField = 0x2,
    JournalReplaying = 0x4,
    JournalDevice = 0x8,
}
bitor_flags!(RequiredFeatureFlags, u32);

/// Feature flags that are required to support writing to a filesystem
enum WritingFeatureFlags { 
    SparseSuperblocksAndGroupDescriptorTables = 0x1,
    FileSize64Bit = 0x2,
    DirectoryContentsBinarySearchTree = 0x4,
}
bitor_flags!(WritingFeatureFlags, u32);

/// The structure of a block group descriptor in the block group descriptor table
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct BlockGroupDescriptor {
    /// The starting-block of the block-usage bitmap
    block_usage_bitmap_addr: BlockAddr,
    /// The starting-block of the inode-usage bitmap
    inode_usage_bitmap_addr: BlockAddr,
    /// The starting-block of the inode table
    inode_table_start_addr: BlockAddr,
    /// The number of free blocks
    unallocated_blocks_count: u16,
    /// The number of free inodes
    unallocated_inodes_count: u16,
    /// The number of inodes allocated for directories
    directories_count: u16,
    _unused: [u8; 14],
}

/// File metadata structure stored in the inode table
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Inode {
    /// The "mode" of the file, low 12 bits are a [`InodePermissions`] bitmask and upper 4
    /// bits are [`InodeType`]
    pub type_and_perms: u16,
    /// Owner user ID
    pub user_id: u16,
    /// File size in bytes, if the filesystem uses 64-bit file sizes, this is the lower 32 bits
    pub size_low: u32,
    /// Last time the file was accessed, in UNIX time
    pub last_access_time: u32,
    /// When the inode was created, in UNIX time
    pub creation_time: u32,
    /// Last time the file was modofied, in UNIX time
    pub last_modification_time: u32,
    /// When the inode was deleted, in UNIX time
    pub deletion_time: u32,
    /// Owner group ID
    pub group_id: u16,
    /// The number of hard-links pointing to this indoe (not symbolic links). When the count reaches
    /// zero the inode and the corresponding data blocks should be unallocated
    pub hard_link_count: u16,
    /// The number of disk sectors (512 bytes) in use by inode's allocated blocks, regardless if the
    /// blocks are actually in use
    pub disk_sector_count: u32,
    /// Bitmask of flags defining how the driver should interact with the inode, as defined
    /// in [`InodeFlags`]
    pub flags: u32,
    /// Value for operating system use
    pub os_specific_1: u32,
    /// Block addresses that directly point to data blocks in use by this inode, where zero means
    /// unused
    pub direct_pointers: [BlockAddr; INODE_DIRECT_PTR_COUNT],
    /// Block address that points to a block of direct pointers, see
    /// [`direct_pointers`](Inode::direct_pointers)
    pub singly_indirect_pointer: BlockAddr,
    /// Block address that points to a block of indirect pointers, see
    /// [`singly_indirect_pointer`](Inode::singly_indirect_pointer)
    pub doubly_indirect_pointer: BlockAddr,
    /// Block address that points to a block of doubly indirect pointers, see
    /// [`doubly_indirect_pointer`](Inode::doubly_indirect_pointer)
    pub triply_indirect_pointer: BlockAddr,
    /// Value indicating the file version, usually unused
    pub generation_number: u32,
    /// The block number of the extended file attributes
    pub extended_attributes_block: BlockAddr,
    /// If the filesystem uses 64-bit file sizes, this is the high 32 bits
    pub size_high: u32,
    /// The block number of the file fragment, usually unused
    pub fragment_block: BlockAddr,
    /// Second value for operating system use
    pub os_specific_2: [u8; 12],
}

impl Inode {
    pub fn get_permisions(&self) -> u16 {
        self.type_and_perms & 0xFFF
    }

    pub fn get_type(&self) -> InodeType {
        (self.type_and_perms >> 12).into()
    }
}

/// The type of file the inode describes
#[derive(PartialEq, Eq)]
pub enum InodeType {
    FIFO = 0x1,
    CharacterDevice = 0x2,
    Directory = 0x4,
    BlockDevice = 0x6,
    RegularFile = 0x8,
    SymbolicLink = 0xA,
    UnixSocket = 0xC,
}

impl From<u16> for InodeType {
    fn from(x: u16) -> Self {
        match x {
            x if x == InodeType::FIFO as u16 => InodeType::FIFO,
            x if x == InodeType::CharacterDevice as u16 => InodeType::CharacterDevice,
            x if x == InodeType::Directory as u16 => InodeType::Directory,
            x if x == InodeType::BlockDevice as u16 => InodeType::BlockDevice,
            x if x == InodeType::RegularFile as u16 => InodeType::RegularFile,
            x if x == InodeType::SymbolicLink as u16 => InodeType::SymbolicLink,
            x if x == InodeType::UnixSocket as u16 => InodeType::UnixSocket,
            _ => unreachable!()
        }
    }
}

/// The permissions assigned to the file the inode describes, can be combined into a bitmask
pub enum InodePermissions {
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

/// Flags defining how the driver should interact with the file the inode describes, can be combined
/// into a bitmask
pub enum InodeFlags {
    SecureDeletion = 0x1,
    RecordForUndelete = 0x2,
    CompressedFile = 0x4,
    SynchronousUpdates = 0x8,
    ImmutableFile = 0x10,
    AppendOnly = 0x20,
    DoNotDump = 0x40,
    NoAccessTime = 0x80,
    Dirty = 0x100,
    CompressedBlocks = 0x200,
    DontUncompress = 0x400,
    CompressionError = 0x800,
    BTreeOrHashIndexedDirectory = 0x1000,
    AFSDirectory = 0x2000,
    Ext3JournalData = 0x4000,
}
bitor_flags!(InodeFlags, u32);

/// Structure of an entry in the table inside a directory's data blocks
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct DirectoryEntry {
    /// Inode number of the file corresponding to this entry
    inode: u32,
    /// Size of the directory entry, i.e. the offset from the start of this entry to the start of
    /// the next entry. This includes the size of the entry structure, the entry name, and any
    /// padding used to align the entries
    size: u16,
    /// Length of the name following the entry structure
    name_length: u8,
    /// The type of the file corresponding to this entry, see [`DirEntryType`]
    type_indicator: DirEntryType,
}

/// The type of the file corresponding to a directory entry
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

/// A parsed Ext2 file system
#[derive(Debug)]
pub struct Ext2Parser<'a> {
    /// The raw file system bytes
    raw_bytes: &'a [u8],
    /// Refernce to the main super block
    super_block: &'a SuperBlock,
    /// Reference to the main super block extended fields
    super_block_extension: &'a SuperBlockExtension,
    /// Reference to the main block group descriptor table
    block_group_descriptor_table: &'a [BlockGroupDescriptor],

    /// Size of a block in bytes
    block_size: usize,
    /// Number of inodes in the filesystem
    inode_count: u32,
    /// Number of blocks in the filesystem
    block_count: u32,
    /// Number of blocks per block group
    blocks_per_block_group: u32,
    /// Number of inodes per block group
    inodes_per_block_group: u32,
    /// Number of block groups
    block_group_count: u32,
    /// The number of pointers that fit in a pointer block
    num_ptrs_per_block: usize,
}

/// Return value of an iteration callback that decides if iteration should continue or end
#[derive(PartialEq, Eq)]
pub enum IterationDecision {
    Continue,
    Break,
}

impl<'a> Ext2Parser<'a> {
    /// Tries to parse the raw bytes of the filesystem
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        // Check that the superblock fits inside the recieved bytes slice
        if bytes.len() < SUPER_BLOCK_OFFSET + SUPER_BLOCK_SIZE {
            return None;
        }

        // Read the super block and verify the Ext2 signature
        let super_block = unsafe { &*(bytes[SUPER_BLOCK_OFFSET..].as_ptr() as *const SuperBlock) };
        if super_block.magic_signature != SUPER_BLOCK_MAGIC_SIGNATURE {
            return None;
        }

        // We dont support the old ext2 revision (major_version = 0)
        if super_block.major_version < 1 {
            return None;
        }

        // Read the extended super block fields
        let extended_fields_offset = SUPER_BLOCK_OFFSET + core::mem::size_of::<SuperBlock>();
        let super_block_extension = unsafe {
            &*(bytes[extended_fields_offset..].as_ptr() as *const SuperBlockExtension)
        };

        // Fail if the filesystem uses a non-standard inode structure
        if super_block_extension.inode_size != core::mem::size_of::<Inode>() as u16 {
            return None;
        }

        // Fail if we don't support any of the required features
        if (super_block_extension.required_feature_flags & !SUPPORTED_REQUIRED_FEATURES_MASK) != 0 {
            return None;
        }

        // We don't support ext2 file systems which don't store a type field in directory entries
        let dir_entry_type_bit = RequiredFeatureFlags::DirectoryEntriesContainTypeField as u32;
        if (super_block_extension.required_feature_flags & dir_entry_type_bit) == 0 {
            return None;
        }

        // Fail if we don't support any of features needed for writing
        if (super_block_extension.writing_feature_flags & !SUPPORTED_WRITING_FEATURES_MASK) != 0 {
            // TODO: Read-only mode
            return None;
        }

        // The block_size_exponent is log2(block_size) - 10, therefore block_size is 1024<<(exp)
        let block_size = 1024usize.checked_shl(super_block.block_size_exponent)?;

        // The block group count could either be calculated using the block count and number of
        // blocks in a block group, or using the inode count and the number of inodes in a block
        // group, so we calculate using both ways and compare as a sanity check. Note that a divide
        // with ceiling-rounding is used because the last block group might contain less blocks
        let block_group_count = 
            div_ceil(super_block.block_count, super_block.num_blocks_in_block_group)?;
        let block_group_count_alt = 
            div_ceil(super_block.inode_count, super_block.num_blocks_in_block_group)?;
        if block_group_count != block_group_count_alt {
            return None;
        }

        // Fail if the byte slice we received does not contain the entire filesystem
        if bytes.len() < block_size.checked_mul(super_block.block_count as usize)? {
            return None;
        }

        // Read the block group descriptor table. The table is located in the block immediately
        // following the super block
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

    /// Returns the next directory entry of the directory with inode number `inode` or `None` if
    /// there are no more directory entries. The current directory entry is determined by the
    /// `opaque_offset` which must be zero for the first entry, and the first item in the returned
    /// tuple for every subsequent call. The returned tuple is of the form 
    /// `(next_opaque_offset, inode, filename, entry_type)`
    pub fn get_next_directory_entry(&self, inode: u32, mut opaque_offset: u32)
        -> Option<(u32, u32, &'a str, DirEntryType)> {
        // Make sure this is actually a directory
        assert!(self.get_inode(inode).get_type() == InodeType::Directory);

        // FIXME: Don't iterate from the start every time
        let mut total_offset: u32 = 0;
        let mut result = None;
        // We iterate through all data blocks, iterating through all directory entries, keeping
        // track of the total offset, until we reach opaque_offset
        self.for_each_data_block(inode, &mut |data_block| {
            let mut curr_offset = 0;
            while curr_offset < self.block_size {
                let dir_entry = unsafe {
                    &*(data_block[curr_offset..].as_ptr() as *const DirectoryEntry)
                };

                // If the directory entries table does not end on a block-border, the rest is zero,
                // so a zero-sized entry means there are no more entries
                if dir_entry.size == 0 {
                    return IterationDecision::Break;
                }

                
                // We reached the offset of the requested (next) entry
                if total_offset == opaque_offset {
                    // If the inode of an entry is zero, it means the entry is unused and we skip it
                    if dir_entry.inode == 0 {
                        opaque_offset += dir_entry.size as u32;
                    } else {
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
                    }
                } else if total_offset > opaque_offset {
                    // If we passed the opaque_offset, it means an invalid offset was passed in,
                    // and there is nothing to do but exit
                    return IterationDecision::Break;
                }

                curr_offset += dir_entry.size as usize;
                total_offset += dir_entry.size as u32;
            }

            IterationDecision::Continue
        });

        result
    }

    /// Calls the `callback` for each entry in the directory whose inode number is `inode`. The
    /// callback will be called with arguments `(inode, filename, entry_type)`
    pub fn for_each_directory_entry<F>(&self, inode: u32, mut callback: F)
        where F: FnMut(u32, &'a str, DirEntryType) -> IterationDecision {
        // Make sure this is really a directory
        assert!(self.get_inode(inode).get_type() == InodeType::Directory);

        self.for_each_data_block(inode, &mut |data_block| {
            let mut curr_offset = 0;
            while curr_offset < self.block_size {
                let dir_entry = unsafe {
                    &*(data_block[curr_offset..].as_ptr() as *const DirectoryEntry)
                };

                // If the directory entries table does not end on a block-border, the rest is zero,
                // so a zero-sized entry means there are no more entries
                if dir_entry.size == 0 {
                    return IterationDecision::Break;
                }

                // If the inode of an entry is zero, it means the entry is unused and we skip it
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

    /// Resolves a path to an inode and directory entry type, if it exists. If the path is relative,
    /// the base directory is specified by the `base_inode`
    pub fn resolve_path_to_inode(&self, path: &str, mut base_inode: u32) -> Option<(u32, DirEntryType)> {
        // The root directory is not handled by the path-walk code, but it has a static inode
        // so we just return it immediately
        if path == "/" {
            return Some((ROOT_INODE, DirEntryType::Directory));
        }

        // If the path starts with a `/` it is an absolute path, and we turn it into a relative path
        // by changing the base inode into the root inode
        let path = if path.starts_with("/") {
            base_inode = ROOT_INODE;
            &path[1..]
        } else {
            path
        };

        // If the path points to a directory, it may end with a `/` so we strip it if it exists. But
        // we need to remember if the path did end with a `/`, because this is illegal if the path
        // does not point to a directory
        let (path, must_be_dir) = if path.ends_with("/") {
            (&path[..path.len()-1], true)
        } else {
            (path, false)
        };
        
        // The current node in the path, starting with the base inode
        let mut inode = base_inode;
        let mut entry_type = DirEntryType::Directory;
        // Boolean that keeps file if we reached a file which is not a directory in the path, which
        // is only allowed to happen once, in the end
        let mut reached_file = false;
        for component in path.split('/') {
            // An empty path component or a path that continues after reaching a file are both
            // invalid
            if component == "" || reached_file {
                return None;
            }

            // We iterate through all files in the directory, trying to find a file with a matching
            // name
            let mut found_match = false;
            self.for_each_directory_entry(inode, |child_inode, child_name, child_type| {
                if child_name == component {
                    inode = child_inode;
                    entry_type = child_type;

                    if child_type == DirEntryType::SymbolicLink {
                        todo!("Handle symbolic links");
                    } else if child_type != DirEntryType::Directory {
                        reached_file = true;
                    }

                    found_match = true;
                    return IterationDecision::Break;
                }

                IterationDecision::Continue
            });

            // If none of the directories children match the component, the requested file does not
            // exist
            if !found_match {
                return None;
            }
        }

        // If the path ended with a `/`, it must be a directory
        if must_be_dir && entry_type != DirEntryType::Directory {
            return None;
        }

        Some((inode, entry_type))
    }

    /// Reads the file with inode number `inode` into `out_buffer`, the amount of bytes read is
    /// returned, and it is limited by the size of `out_buffer`
    pub fn get_contents(&self, inode: u32, out_buffer: &mut [u8]) -> usize {
        self.get_contents_with_offset(inode, out_buffer, 0)
    }

    /// Reads the file with inode number `inode` into `out_buffer` starting at the specified offset.
    /// The amount of bytes read is returned, and it is limited by the size of `out_buffer`
    pub fn get_contents_with_offset(&self, inode: u32, out_buffer: &mut [u8], offset: usize) -> usize {
        if out_buffer.len() == 0 {
            return 0;
        }

        // FIXME: Don't iterate from the start every time...

        let inode_metadata = self.get_inode(inode);
        let file_size = inode_metadata.size_low as usize; // TODO: 64bit size

        // total_read tracks the number of bytes we read into the buffer, and data_offset tracks the
        // number of bytes we went over from the start of the file
        let mut total_read = 0;
        let mut data_offset = 0;
        self.for_each_data_block(inode, &mut |data_block| {
            // The last block may be less than the normal size if the block size does not divide the
            // file size
            let block_length = data_block.len().min(file_size - data_offset);

            // We check if part of this block is after the requested offset
            if offset < data_offset + block_length {
                // We might need to read from the middle of the first block we read
                let block_offset = if offset > data_offset {
                    offset - data_offset
                } else {
                    0
                };

                
                // The amount of bytes we need to read is the minimum between the number of bytes
                // in the block we are interested in, and the space left in the out buffer
                let left_in_block = block_length - block_offset;
                let size_left = left_in_block.min(out_buffer.len() - total_read);

                out_buffer[total_read..total_read+size_left].copy_from_slice(&data_block[..size_left]);
                total_read += size_left;

                // If we reached the end of the out buffer, we can finish
                if total_read == out_buffer.len() {
                    return IterationDecision::Break;
                }
            }

            data_offset += self.block_size;

            // If we reached the end of the logical file there is no need to continue
            if data_offset >= file_size {
                IterationDecision::Break
            } else {
                IterationDecision::Continue
            }
        });

        total_read
    }

    /// Calls the `callback` for each block allocated to inode whose number is `inode`. The callback
    /// will be called with a byte slice of the block's content
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

    /// Returns a reference to the inode metadata structure of the inode whose number is `inode`
    pub fn get_inode(&self, inode: u32) -> &'a Inode {
        // Inode numbers are start at 1
        assert!(inode >= 1);
        assert!(inode <= self.inode_count);

        // We calculate the block group of the inode, and the index inside the block group
        let block_group = ((inode - 1) / self.inodes_per_block_group) as usize;
        let inode_index = ((inode - 1) % self.inodes_per_block_group) as usize;

        // The block group table contains the block address of the inode table of the block group
        let inode_table_block_addr = 
            self.block_group_descriptor_table[block_group].inode_table_start_addr.0 as usize;
        let inode_offset = 
            (inode_table_block_addr * self.block_size) + (inode_index * core::mem::size_of::<Inode>());
        
        unsafe { 
            &*(self.raw_bytes[inode_offset..].as_ptr() as *const Inode)
        }
    }

    /// Returns a byte slice of the data of the block at address `block`
    fn get_block(&self, block: BlockAddr) -> &'a [u8] {
        let offset = block.0 as usize * self.block_size;
        &self.raw_bytes[offset..offset+self.block_size]
    }

    // Returns a slice of the pointers inside the block at address `block`
    fn get_ptrs_block(&self, block: BlockAddr) -> &'a [BlockAddr] {
        unsafe { 
            core::slice::from_raw_parts(
                self.get_block(block).as_ptr() as *const BlockAddr,
                self.num_ptrs_per_block
            )
        }
    }

    /// Calls the `callback` for each block pointed to by the pointers in the pointer block `block`.
    /// The callback will be called with a byte slice of the block's content
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

    /// Calls the `callback` for each block eventually pointed to by the pointers in the indirect
    /// pointers block `block`. The callback will be called with a byte slice of the block's content
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

    /// Calls the `callback` for each block eventually pointed to by the pointers in the doubly
    /// indirect pointers block `block`. The callback will be called with a byte slice of the
    /// block's content
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

/// Calculates the integer division `x/y` while rounding towards the ceiling. Returns `None` if y is
/// zero
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
