use byteorder::{LittleEndian, ReadBytesExt};
use custom_debug_derive::Debug as CustumDebug;
use failure::Fallible;
use num_enum::*;
use positioned_io::{Cursor, ReadAt, Slice};
use std::convert::TryFrom;
use std::fs::OpenOptions;

type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    //Open partition read only
    let file = OpenOptions::new()
        .read(true)
        .open("/dev/ubuntu-vg/ubuntu-lv")?;
    //open the superblock
    let sb = Superblock::new(&file)?;
    println!("{:#?}", sb);
    //get inode for /
    let root_inode = InodeNumber(2).inode(&sb, &file)?;
    println!("({:?}) {:#?}", root_inode.filetype(), root_inode);
    //goes down to /etc
    let etc_inode = root_inode
        .child("etc", &sb, &file)?
        .expect("/etc is real?")
        .inode(&sb, &file)?;
    println!("{:?} {:#?}", etc_inode.filetype(), etc_inode);
    //find /etc/hosts
    let hosts_inode = etc_inode
        .child("hosts", &sb, &file)?
        .expect("/etc/hosts is real?")
        .inode(&sb, &file)?;
    println!("{:?} {:#?}", hosts_inode.filetype(), hosts_inode);
    //gets the data
    let hosts_data = hosts_inode.data(&sb, &file)?;
    //read into buffer
    let hosts_data = Reader::new(&hosts_data).vec(0, hosts_inode.size as usize)?;
    //read into string
    let hosts_data = String::from_utf8_lossy(&hosts_data);
    println!("{}", hosts_data);

    Ok(())
}

#[derive(CustumDebug)]
struct DirectoryEntry {
    #[debug(skip)]
    len: u64,
    inode: InodeNumber,
    name: String,
}

impl DirectoryEntry {
    //makes a directory entry
    // its just its inode
    // its len
    // and its name(for readability)
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        let name_len = r.u8(0x6)? as usize;
        Ok(Self {
            inode: InodeNumber(r.u32(0x0)? as u64),
            len: r.u16(0x4)? as u64,
            name: String::from_utf8_lossy(&r.vec(0x8, name_len)?).into(),
        })
    }
}
//it helps to know where we are in the header
#[derive(Debug)]
struct Extent {
    len: u64,
    start: u64,
}

impl Extent {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            len: r.u16(0x4)? as u64,
            //the block number the extent points
            // its split in upper 16bits and lower 32bits
            start: ((r.u16(0x6)? as u64) << 32) + r.u32(0x8)? as u64,
        })
    }
}

#[derive(Debug)]
struct ExtentHeader {
    entries: u64,
    depth: u64,
}

impl ExtentHeader {
    //a slice with the depth of the extent
    //and its entries
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        let magic = r.u16(0x0)?;
        assert_eq!(magic, 0xF30A);

        Ok(Self {
            entries: r.u16(0x2)? as u64,
            depth: r.u16(0x6)? as u64,
        })
    }
}
//Enumerate the fyletypes by mode
//only  ext4(i didnt check)
#[derive(Debug, TryFromPrimitive)]
#[repr(u16)]
enum Filetype {
    Fifo = 0x1000,
    CharacterDevice = 0x2000,
    Directory = 0x4000,
    BlockDevice = 0x6000,
    Reguler = 0x8000,
    SymbolicLink = 0xA000,
    SOCKET = 0xC000,
}

#[derive(Debug, Clone, Copy)]
struct InodeNumber(u64);

impl InodeNumber {
    //where you are kindoff
    fn blockgroup_number(self, sb: &Superblock) -> BlockGroupNumber {
        let n = (self.0 - 1) / sb.inodes_per_group;
        BlockGroupNumber(n)
    }
    //gets inode data
    fn inode_slice<T>(self, sb: &Superblock, dev: T) -> Result<Slice<T>>
    where
        T: ReadAt,
    {
        let desc = self.blockgroup_number(sb).desc(sb, &dev)?;
        let table_off = desc.inode_table * sb.block_size;
        let idx_in_table = (self.0 - 1) % sb.inodes_per_group;
        let inode_off = table_off + sb.inode_size * idx_in_table;
        Ok(Slice::new(dev, inode_off, Some(sb.inode_size)))
    }
    //builds inode
    fn inode(self, sb: &Superblock, dev: &dyn ReadAt) -> Result<Inode> {
        let slice = self.inode_slice(sb, dev)?;
        Inode::new(&slice)
    }
}

#[derive(CustumDebug)]
struct Inode {
    #[debug(format = "{:o}")]
    mode: u16,
    size: u64,
    #[debug(skip)]
    block: Vec<u8>,
}

impl Inode {
    //build inode
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            mode: r.u16(0x0)?,
            size: r.u64_lohi(0x4, 0x6c)?,
            block: r.vec(0x28, 60)?,
        })
    }
    //test the filetype
    fn filetype(&self) -> Filetype {
        Filetype::try_from(self.mode & 0xF000).unwrap()
    }
    //gets the data in the inode
    fn data<T>(&self, sb: &Superblock, dev: T) -> Result<Slice<T>>
    where
        T: ReadAt,
    {
        let ext_header = ExtentHeader::new(&Slice::new(&self.block, 0, Some(12)))?;
        assert_eq!(ext_header.depth, 0);
        assert_eq!(ext_header.entries, 1);

        let ext = Extent::new(&Slice::new(&self.block, 12, Some(12)))?;
        assert_eq!(ext.len, 1);

        let offset = ext.start * sb.block_size;
        let len = ext.len * sb.block_size;
        Ok(Slice::new(dev, offset, Some(len)))
    }
    //read dir entries from inode
    fn dir_entries(&self, sb: &Superblock, dev: &dyn ReadAt) -> Result<Vec<DirectoryEntry>> {
        let data = self.data(sb, dev)?;

        let mut entries = Vec::new();
        let mut offset: u64 = 0;
        loop {
            let entry = DirectoryEntry::new(&Slice::new(&data, offset, None))?;
            if entry.inode.0 == 0 {
                break;
            }
            offset += entry.len;
            entries.push(entry);
        }
        Ok(entries)
    }
    //get inode childs
    //traverse the fs
    fn child(&self, name: &str, sb: &Superblock, dev: &dyn ReadAt) -> Result<Option<InodeNumber>> {
        let entries = self.dir_entries(sb, dev)?;
        Ok(entries
            .into_iter()
            .filter(|x| x.name == name)
            .map(|x| x.inode)
            .next())
    }
}

#[derive(Debug)]
struct BlockGroupDescriptor {
    inode_table: u64,
}

impl BlockGroupDescriptor {
    fn new(slice: &dyn ReadAt) -> Result<Self> {
        let r = Reader::new(slice);
        Ok(Self {
            inode_table: r.u64_lohi(0x8, 0x28)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockGroupNumber(u64);

impl BlockGroupNumber {
    //read it to a slice
    fn desc_slice<T>(self, sb: &Superblock, dev: T) -> Slice<T>
    where
        T: ReadAt,
    {
        //checks if different from superblock
        assert!(sb.block_size != 1024, "1024 block size not");
        //superblock = 1 block
        let gdt_start = 1 * sb.block_size;
        let offset = gdt_start + self.0 * BlouckGroupDescriptor::SIZE;
        Slice::new(dev, offset, None)
    }
    //gets the groupdescriptor directly
    fn desc(self, sb: &Superblock, dev: &dyn ReadAt) -> Result<BlockGroupDescriptor> {
        let slice = self.desc_slice(sb, dev);
        BlockGroupDescriptor::new(&slice)
    }
}

struct BlouckGroupDescriptor {}

impl BlouckGroupDescriptor {
    const SIZE: u64 = 64;
}

#[derive(CustumDebug)]
struct Superblock {
    #[debug(format = "{:x}")]
    magic: u16,
    block_size: u64,
    blocks_per_group: u64,
    inodes_per_group: u64,
    inode_size: u64,
}

impl Superblock {
    fn new(dev: &dyn ReadAt) -> Result<Self> {
        //read the superblock
        let r = Reader::new(Slice::new(dev, 1024, None));
        //return in a struct
        Ok(Self {
            magic: r.u16(0x38)?,
            block_size: (2u32.pow(10 + r.u32(0x18)?)) as u64,
            blocks_per_group: r.u32(0x20)? as u64,
            inodes_per_group: r.u32(0x28)? as u64,
            inode_size: r.u16(0x58)? as u64,
        })
    }
}
//if it has ReadAt
// we can read it
struct Reader<IO: ReadAt> {
    inner: IO,
}
// you pass an offset
// it implements ReadAt
// return a Fallible<T>
impl<IO: ReadAt> Reader<IO> {
    fn new(inner: IO) -> Self {
        Self { inner }
    }

    fn u8(&self, offset: u64) -> Fallible<u8> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u8().unwrap())
    }

    fn u16(&self, offset: u64) -> Fallible<u16> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u16::<LittleEndian>()?)
    }

    fn u32(&self, offset: u64) -> Fallible<u32> {
        let mut cursor = Cursor::new_pos(&self.inner, offset);
        Ok(cursor.read_u32::<LittleEndian>()?)
    }
    //read the 32bits at the bottom & the 32 bits at the top
    fn u64_lohi(&self, lo: u64, hi: u64) -> Fallible<u64> {
        Ok(self.u32(lo)? as u64 + ((self.u32(hi)? as u64) << 32))
    }
    //Creates a vec at offset of 0u8 for len
    // offset & len are passed to it
    fn vec(&self, offset: u64, len: usize) -> Fallible<Vec<u8>> {
        let mut v = vec![0u8; len];
        self.inner.read_exact_at(offset, &mut v)?;
        Ok(v)
    }
}
