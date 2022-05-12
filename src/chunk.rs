use crc::{Crc, CRC_32_ISO_HDLC};
use std::{
    error,
    fmt::{Display},
    str::{self, FromStr},
};

use crate::{chunk_type::ChunkType, Error, Result};

pub struct Chunk {
    length: u32,
    chunk_type: ChunkType,
    chunk_data: Vec<u8>,
    crc: u32,
}

#[derive(Debug)]
pub struct InvalidCrcError;

impl Chunk {
    pub fn new(chunk_type: ChunkType, data: Vec<u8>) -> Self {
        let crc = Self::calculate_crc(&chunk_type, &data);

        Self {
            length: data.len() as u32,
            chunk_type,
            chunk_data: data,
            crc,
        }
    }

    fn length(&self) -> u32 {
        self.length
    }

    fn crc(&self) -> u32 {
        self.crc
    }

    fn chunk_type(&self) -> &ChunkType {
        &self.chunk_type
    }

    fn data(&self) -> &[u8] {
        &self.chunk_data
    }

    fn data_as_string(&self) -> Result<String> {
        Ok(str::from_utf8(&self.chunk_data).unwrap().to_string())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        // this code is the same as the one used in testing_chunk() in the unit tests
        self.length
            .to_be_bytes()
            .iter()
            .chain(self.chunk_type.bytes().iter())
            .chain(self.chunk_data.iter())
            .chain(self.crc.to_be_bytes().iter())
            .copied()
            .collect::<Vec<u8>>()
    }

    pub fn calculate_crc(chunk_type: &ChunkType, data: &Vec<u8>) -> u32 {
        /*
            from http://www.libpng.org/pub/png/spec/1.2/PNG-Structure.html#Chunk-layout

            the crc is calculated on the bytes of the chunk type and data, and it needs to be 4 bytes long

            I had to try out pretty much all of the 32 bit algorithms available in the crc crate, until I found the one that works with the provided test
        */
        let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);

        // imperative way by manually iterating over the two sequences
        /*let mut bytes = Vec::<u8>::new();

        for b in self.chunk_type.bytes() {
            bytes.push(b);
        }

        for b in &self.data {
            bytes.push(*b);
        }*/

        // functional way by chaining the two iterators together and collecting them in a new Vec at the end
        crc.checksum(
            &chunk_type
                .bytes()
                .iter()
                .cloned()
                .chain(data.iter().cloned())
                .collect::<Vec<u8>>(),
        )
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chunk {{",)?;
        writeln!(f, "  Length: {}", self.length())?;
        writeln!(f, "  Type: {}", self.chunk_type())?;
        writeln!(f, "  Data: {} bytes", self.data().len())?;
        writeln!(f, "  Crc: {}", self.crc())?;
        writeln!(f, "}}",)?;
        Ok(())
    }
}

impl TryFrom<&[u8]> for Chunk {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        /*
            a slice of u8 (byte) interpreted as a png chunk is structured as follows:
            - first 4 bytes: length (n)
            - next 4 bytes: chunk type
            - next n bytes: chunk data
            - last 4 bytes: crc
        */

        // the length and crc are encoded as big endian bytes, so they must be read like this
        let length = u32::from_be_bytes(value[..4].try_into().unwrap());
        let chunk_type = ChunkType::from_str(str::from_utf8(&value[4..8]).unwrap()).unwrap();

        let data_end_index = 8 + length as usize;
        let chunk_data = str::from_utf8(&value[8..data_end_index])
            .unwrap()
            .as_bytes()
            .to_vec();

        let input_crc = u32::from_be_bytes(value[data_end_index..].try_into().unwrap());
        let crc = Self::calculate_crc(&chunk_type, &chunk_data);

        if crc != input_crc {
            return Err(Box::new(InvalidCrcError));
        }

        Ok(Chunk {
            length,
            chunk_type,
            chunk_data,
            crc,
        })
    }
}

impl error::Error for InvalidCrcError {}

impl Display for InvalidCrcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A valid CRC must match the one that is calculated again upon creating a Chunk"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_type::ChunkType;
    use std::str::FromStr;

    #[test]
    fn test_new_chunk() {
        let chunk_type = ChunkType::from_str("RuSt").unwrap();
        let data = "This is where your secret message will be!"
            .as_bytes()
            .to_vec();
        let chunk = Chunk::new(chunk_type, data);

        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_chunk_length() {
        let chunk = testing_chunk();

        assert_eq!(chunk.length(), 42);
    }

    #[test]
    fn test_chunk_type() {
        let chunk = testing_chunk();

        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
    }

    #[test]
    fn test_chunk_string() {
        let chunk = testing_chunk();
        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");

        assert_eq!(chunk_string, expected_chunk_string);
    }

    #[test]
    fn test_chunk_crc() {
        let chunk = testing_chunk();

        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_valid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;
        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();
        let chunk = Chunk::try_from(chunk_data.as_ref()).unwrap();
        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");

        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
        assert_eq!(chunk_string, expected_chunk_string);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_invalid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656333;
        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();
        let chunk = Chunk::try_from(chunk_data.as_ref());

        assert!(chunk.is_err());
    }

    #[test]
    pub fn test_chunk_trait_impls() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;
        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();
        let chunk: Chunk = TryFrom::try_from(chunk_data.as_ref()).unwrap();
        let _chunk_string = format!("{}", chunk);
    }

    fn testing_chunk() -> Chunk {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;
        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        Chunk::try_from(chunk_data.as_ref()).unwrap()
    }
}
