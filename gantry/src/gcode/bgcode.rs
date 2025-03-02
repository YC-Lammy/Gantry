// Binary Gcode

use std::io::SeekFrom;

use tokio::{fs::File, io::AsyncSeekExt};

#[repr(u16)]
pub enum Error {
    Success,
    ReadError,
    WriteError,
    InvalidMagicNumber,
    InvalidVersionNumber,
    InvalidChecksumType,
    InvalidBlockType,
    InvalidCompressionType,
    InvalidMetadataEncodingType,
    InvalidGCodeEncodingType,
    DataCompressionError,
    DataUncompressionError,
    MetadataEncodingError,
    MetadataDecodingError,
    GCodeEncodingError,
    GCodeDecodingError,
    BlockNotFound,
    InvalidChecksum,
    InvalidThumbnailFormat,
    InvalidThumbnailWidth,
    InvalidThumbnailHeight,
    InvalidThumbnailDataSize,
    InvalidBinaryGCodeFile,
    InvalidAsciiGCodeFile,
    InvalidSequenceOfBlocks,
    InvalidBuffer,
    AlreadyBinarized,
    MissingPrinterMetadata,
    MissingPrintMetadata,
    MissingSlicerMetadata,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    None,
    CRC32,
}

#[repr(u16)]
pub enum BlockType {
    FileMetadata,
    GCode,
    SlicerMetadata,
    PrinterMetadata,
    PrintMetadata,
    Thumbnail,
}

#[repr(u16)]
pub enum CompressionType {
    None,
    Deflate,
    Heatshrink11_4,
    Heatshrink12_4,
}

#[repr(u16)]
pub enum MetadataEncodingType {
    INI,
}

#[repr(u16)]
pub enum GCodeEncodingType {
    None,
    MeatPack,
    MeatPackComments,
}

#[repr(u16)]
pub enum ThumbnailFormat {
    PNG,
    JPG,
    QOI,
}

struct FileHeader {
    /// GCDE
    pub magic: u32,
    /// Version of the G-code binarization
    pub version: u32,
    /// Algorithm used for checksum
    pub checksum_type: u16,
}

impl FileHeader {
    pub fn new(magic: u32, version: u32, checksum_type: u16) -> Self {
        Self {
            magic,
            version,
            checksum_type,
        }
    }
}

pub struct BlockHeader {
    pub type_: u16,
    pub compression: u16,
    pub uncompressed_size: u32,
    pub compressed_size: u32,
    position: usize,
}

impl BlockHeader {
    pub fn get_position(&self) -> usize{
        self.position
    }

    pub fn get_size(&self) -> u32{
        if self.compression == CompressionType::None as u16{
            return self.uncompressed_size
        }

        return self.compressed_size
    }
}

pub struct ThumbnailParams {
    pub format: u16,
    pub width: u16,
    pub height: u16,
}

pub struct CheckSum{
    ty: ChecksumType
}

impl CheckSum{
    pub fn new(ty: ChecksumType) -> Self{
        Self{
            ty
        }
    }

    pub fn get_type(&self) -> ChecksumType{
        self.ty
    }
}

async fn verify_block_checksum(file: &mut File, file_header: &FileHeader, block_header: &BlockHeader, buffer: &mut [u8]) -> Result<(), Error>{
    if buffer.len() == 0{
        return Err(Error::InvalidBuffer)
    }

    if file_header.checksum_type == ChecksumType::None as u16{
        return Ok(())
    }

    if file.seek(SeekFrom::Start(block_header.get_position() as u64 + block_header.get_size() as u64)).await.is_err(){
        return Err(Error::ReadError)
    }

    todo!()
}