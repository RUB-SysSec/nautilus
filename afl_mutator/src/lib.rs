extern crate byteorder;
extern crate rand;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use rand::{thread_rng, Rng};
use std::cmp;
use std::io::Cursor;
use std::ops::Range;

//Constants for the Arith stages
const AFL_ARITH_MAX: usize = 35;

//Constants for the Havoc stages
const AFL_HAVOC_BLK_LARGE: usize = 1500;
//const AFL_HAVOC_MIN:u16 = 2000;
const AFL_HAVOC_STACK_POW2: u8 = 7;

//Constants for the interest stages
static INTERESTING_8_BIT: [u8; 9] = [
    128, /*-128*/
    255, /*-1*/
    0, 1, 16, 32, 64, 100, 127,
];
static INTERESTING_16_BIT: [u16; 10] = [
    32768, /*-32768*/
    65407, /*-129*/
    128, 255, 256, 512, 1000, 1024, 4096, 32767,
];
static INTERESTING_32_BIT: [u32; 8] = [
    2147483648, /*-2147483648*/
    4194304250, /*-100663046*/
    4294934527, /*-32769*/
    32768, 65535, 65536, 100663045, 2147483647,
];

//Check if bitflip for one byte
macro_rules! is_not_bitflip {
    (change => $change:expr) => {{
        let is_bitflip = match $change >> $change.trailing_zeros() {
            0b00000001 => false,
            0b00000011 => false,
            0b00001111 => false,
            0b11111111 => false,
            _ => true,
        };
        is_bitflip
    }};
}

//Check if bitflip for two bytes
macro_rules! is_not_bitflip_two_bytes {
    (change => $change:expr) => {{
        let is_bitflip = match $change >> $change.trailing_zeros() {
            0b0000000000000001 => false,
            0b0000000000000011 => false,
            0b0000000000001111 => false,
            0b0000000011111111 => false,
            0b1111111111111111 => false,
            _ => true,
        };
        is_bitflip
    }};
}

// Fuzzing stages
pub enum Stage {
    /*1*/ Flip1 {
        offset: usize,
    },
    /*2*/ Flip2 {
        offset: usize,
    },
    /*3*/ Flip4 {
        offset: usize,
    },
    /*4*/ Flip8 {
        offset: usize,
    },
    /*5*/ Flip16 {
        offset: usize,
    },
    /*6*/ Flip32 {
        offset: usize,
    },
    /*7*/ Arith8 {
        offset: usize,
        value: i8,
    },
    /*8*/
    Arith16 {
        offset: usize,
        value: i16,
        endianess: bool,
    },
    /*9*/
    Arith32 {
        offset: usize,
        value: i32,
        endianess: bool,
    },
    /*10*/ Interest8 {
        offset: usize,
        value: u8,
    },
    /*11*/
    Interest16 {
        offset: usize,
        value: u8,
        endianess: bool,
    },
    /*12*/
    Interest32 {
        offset: usize,
        value: u8,
        endianess: bool,
    },
    /*13*/	//ExtrasUI{offset: usize},
    /*14*/	//ExtrasAO{offset: usize},
    /*15*/	//Havoc{offset: u16},
    /*16*/  //Splice{offset: usize}
    /*17*/
    Bruteforce {
        offset: usize,
        value: u8,
    },
    Finished,
}
//Options
pub struct Options {
    change_size: bool,
}
//Return values of mutation steps
enum ReturnValue {
    ChangedBits { range: Range<usize> }, //Return value after successful mutation
    Skip,                                //If step is skipped
    Finished,                            //If deterministic is finished
}
//Saves the current mutation state
pub struct MutationState {
    effector_map: Vec<u8>,
    stage: Stage,
    options: Options,
}

impl MutationState {
    pub fn new_bitflip(vec: Vec<u8>) -> MutationState {
        MutationState {
            options: Options { change_size: false },
            stage: Stage::Flip1 { offset: 0 },
            effector_map: vec,
        }
    }
    pub fn new_deterministic(vec: Vec<u8>) -> MutationState {
        MutationState {
            options: Options { change_size: false },
            stage: Stage::Arith8 {
                offset: 0,
                value: -(AFL_ARITH_MAX as i8),
            },
            effector_map: vec,
        }
    }
    //Change effector map
    pub fn new_effector(&mut self, vec: Vec<u8>) {
        self.effector_map = vec;
    }

    //Do all the flips!
    pub fn deterministic_flip_bits(&mut self, data: &mut Vec<u8>) -> Option<Range<usize>> {
        let mut restore: ReturnValue;
        loop {
            restore = match self.stage {
                Stage::Flip1 { offset } => self.flip1(offset, data),
                Stage::Flip2 { offset } => self.flip2(offset, data),
                Stage::Flip4 { offset } => self.flip4(offset, data),
                Stage::Flip8 { offset } => self.flip8(offset, data),
                Stage::Flip16 { offset } => self.flip16(offset, data),
                Stage::Flip32 { offset } => self.flip32(offset, data),
                _ => self.finished(),
            };
            match restore {
                ReturnValue::Skip => {} //Do nothing
                ReturnValue::ChangedBits { range } => return Some(range),
                ReturnValue::Finished => return None,
            };
        }
    }
    //Do the next mutation step
    pub fn deterministic(&mut self, data: &mut Vec<u8>) -> Option<Range<usize>> {
        let mut restore: ReturnValue;
        loop {
            restore = match self.stage {
                Stage::Flip1 { offset } => self.flip1(offset, data),
                Stage::Flip2 { offset } => self.flip2(offset, data),
                Stage::Flip4 { offset } => self.flip4(offset, data),
                Stage::Flip8 { offset } => self.flip8(offset, data),
                Stage::Flip16 { offset } => self.flip16(offset, data),
                Stage::Flip32 { offset } => self.flip32(offset, data),
                Stage::Arith8 { offset, value } => self.arith8(offset, value, data),
                Stage::Arith16 {
                    offset,
                    value,
                    endianess,
                } => self.arith16(offset, value, endianess, data),
                Stage::Arith32 {
                    offset,
                    value,
                    endianess,
                } => self.arith32(offset, value, endianess, data),
                Stage::Interest8 { offset, value } => self.interest8(offset, value, data),
                Stage::Interest16 {
                    offset,
                    value,
                    endianess,
                } => self.interest16(offset, value, endianess, data),
                Stage::Interest32 {
                    offset,
                    value,
                    endianess,
                } => self.interest32(offset, value, endianess, data),
                Stage::Bruteforce { offset, value } => self.bruteforce(offset, value, data),
                Stage::Finished => self.finished(),
            };
            match restore {
                ReturnValue::Skip => {} //Do nothing
                ReturnValue::ChangedBits { range } => return Some(range),
                ReturnValue::Finished => return None,
            };
        }
    }

    //Start Brutefoce at byte at offset
    pub fn startbruteforce(&mut self, offset: usize) {
        self.stage = Stage::Bruteforce { offset, value: 0 };
    }
    //Do one random permutation
    pub fn havoc(&mut self, data: &mut Vec<u8>) {
        let mut rng = thread_rng();
        let max_step: u8;
        let mut datalen;
        if self.options.change_size {
            max_step = 15;
        } else {
            max_step = 12;
        }

        for _ in 0..(1 << 1 + rng.gen_range(0, AFL_HAVOC_STACK_POW2)) {
            datalen = data.len();
            match rng.gen_range(0, max_step) {
                0 => self.flip_random(data, datalen), //Flip a single bit somewhere
                1 => self.set_random_interest8(data, datalen), //Set byte to interesting value
                2 => self.set_random_interest16(data, datalen), //Set word to interesting value
                3 => self.set_random_interest32(data, datalen), //Set dword to interesting value
                4 => self.subtract_random_arith8(data, datalen), //Randomly subtract from byte
                5 => self.add_random_arith8(data, datalen), //Randomly add to byte
                6 => self.subtract_random_arith16(data, datalen), //Randomly subtract from word
                7 => self.add_random_arith16(data, datalen), //Randomly add to word
                8 => self.subtract_random_arith32(data, datalen), //Randomly subtract from dword
                9 => self.add_random_arith32(data, datalen), //Randomly add to dword
                10 => self.set_random_byte(data, datalen), //Set Random byte to random value
                11 => self.overwrite_random_bytes(data, datalen), //Overwrite bytes with randomly selected chunk (75%) or fixed bytes (25%)
                12...13 => self.delete_random_bytes(data, datalen), //Delete bytes
                14 => self.insert_random_bytes(data, datalen), //75% clone bytes, 25% insert random byte
                _ => {}
            };
        }
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip1(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip1 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 1 < datalen * 8 {
                        self.stage = Stage::Flip1 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Flip2 { offset: 0 };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 8 < datalen * 8 {
                        self.stage = Stage::Flip1 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip2
                    else {
                        self.stage = Stage::Flip2 { offset: 0 };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip1 was called while not in a Flip1 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip2(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip2 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 2 < datalen * 8 {
                        self.stage = Stage::Flip2 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Flip4 { offset: 0 };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 9 < datalen * 8 {
                        self.stage = Stage::Flip2 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip4
                    else {
                        self.stage = Stage::Flip4 { offset: 0 };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip2 was called while not in a Flip2 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip4(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip4 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 4 < datalen * 8 {
                        self.stage = Stage::Flip4 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Flip8 { offset: 0 };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 11 < datalen * 8 {
                        self.stage = Stage::Flip4 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip8
                    else {
                        self.stage = Stage::Flip8 { offset: 0 };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip4 was called while not in a Flip4 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip8(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip8 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 8 < datalen * 8 {
                        self.stage = Stage::Flip8 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Flip16 { offset: 0 };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 15 < datalen * 8 {
                        self.stage = Stage::Flip8 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip16
                    else {
                        self.stage = Stage::Flip16 { offset: 0 };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip8 was called while not in a Flip8 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip16(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip16 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 16 < datalen * 8 {
                        self.stage = Stage::Flip16 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Flip32 { offset: 0 };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 23 < datalen * 8 {
                        self.stage = Stage::Flip16 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip16
                    else {
                        self.stage = Stage::Flip32 { offset: 0 };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip16 was called while not in a Flip16 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_flip32(&mut self, offset: usize, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Flip32 { .. } => {
                if effector {
                    //If bit offset has to increase
                    if offset + 32 < datalen * 8 {
                        self.stage = Stage::Flip32 { offset: offset + 1 };
                    }
                    //If stage 1 is completed
                    else {
                        self.stage = Stage::Arith8 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i8),
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 39 < datalen * 8 {
                        self.stage = Stage::Flip16 { offset: offset + 8 };
                    }
                    //If in last byte to to stage Flip16
                    else {
                        self.stage = Stage::Arith8 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i8),
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("flip16 was called while not in a Flip16 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_arith8(&mut self, offset: usize, value: i8, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Arith8 { .. } => {
                if effector {
                    //If value has to increase
                    if value < (AFL_ARITH_MAX as i8) {
                        self.stage = Stage::Arith8 {
                            offset,
                            value: value + 1,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 1 < datalen {
                        self.stage = Stage::Arith8 {
                            offset: offset + 1,
                            value,
                        };
                    }
                    //If stage arith8 is completed
                    else {
                        self.stage = Stage::Arith16 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i16),
                            endianess: true,
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 1 < datalen {
                        self.stage = Stage::Arith8 {
                            offset: offset + 1,
                            value: -(AFL_ARITH_MAX as i8),
                        };
                    }
                    //If in last byte to to stage Arith16
                    else {
                        self.stage = Stage::Arith16 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i16),
                            endianess: true,
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("arith8 was called while not in a Arith8 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_arith16(
        &mut self,
        offset: usize,
        value: i16,
        endianess: bool,
        datalen: usize,
        effector: bool,
    ) {
        match self.stage {
            //Increase state
            Stage::Arith16 { .. } => {
                if effector {
                    //If value has to increase
                    if value < (AFL_ARITH_MAX as i16) {
                        self.stage = Stage::Arith16 {
                            offset,
                            value: value + 1,
                            endianess,
                        };
                    }
                    //If endianess has to change
                    else if endianess {
                        self.stage = Stage::Arith16 {
                            offset,
                            value: -(AFL_ARITH_MAX as i16),
                            endianess: false,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 2 < datalen {
                        self.stage = Stage::Arith16 {
                            offset: offset + 1,
                            value,
                            endianess: true,
                        };
                    }
                    //If stage arith16 is completed
                    else {
                        self.stage = Stage::Arith32 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i32),
                            endianess: true,
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 2 < datalen {
                        self.stage = Stage::Arith16 {
                            offset: offset + 1,
                            value: -(AFL_ARITH_MAX as i16),
                            endianess: true,
                        };
                    }
                    //If in last byte, go to stage Arith32
                    else {
                        self.stage = Stage::Arith32 {
                            offset: 0,
                            value: -(AFL_ARITH_MAX as i32),
                            endianess: true,
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("arith8 was called while not in a Arith8 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_arith32(
        &mut self,
        offset: usize,
        value: i32,
        endianess: bool,
        datalen: usize,
        effector: bool,
    ) {
        match self.stage {
            //Increase state
            Stage::Arith32 { .. } => {
                if effector {
                    //If value has to increase
                    if value < (AFL_ARITH_MAX as i32) {
                        self.stage = Stage::Arith32 {
                            offset,
                            value: value + 1,
                            endianess,
                        };
                    }
                    //If endianess has to change
                    else if endianess {
                        self.stage = Stage::Arith32 {
                            offset,
                            value: -(AFL_ARITH_MAX as i32),
                            endianess: false,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 4 < datalen {
                        self.stage = Stage::Arith32 {
                            offset: offset + 1,
                            value,
                            endianess: true,
                        };
                    }
                    //If stage arith16 is completed
                    else {
                        self.stage = Stage::Interest8 {
                            offset: 0,
                            value: 0,
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 4 < datalen {
                        self.stage = Stage::Arith32 {
                            offset: offset + 1,
                            value: -(AFL_ARITH_MAX as i32),
                            endianess: true,
                        };
                    }
                    //If in last byte, go to stage Interest8
                    else {
                        self.stage = Stage::Interest8 {
                            offset: 0,
                            value: 0,
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("arith32 was called while not in a Arith32 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_interest8(&mut self, offset: usize, value: u8, datalen: usize, effector: bool) {
        match self.stage {
            //Increase state
            Stage::Interest8 { .. } => {
                if effector {
                    //If value has to increase
                    if value + 1 < (INTERESTING_8_BIT.len() as u8) {
                        self.stage = Stage::Interest8 {
                            offset,
                            value: value + 1,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 1 < datalen {
                        self.stage = Stage::Interest8 {
                            offset: offset + 1,
                            value,
                        };
                    }
                    //If stage Interest8 is completed
                    else {
                        self.stage = Stage::Interest16 {
                            offset: 0,
                            value: 0,
                            endianess: true,
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 1 < datalen {
                        self.stage = Stage::Interest8 {
                            offset: offset + 1,
                            value: 0,
                        };
                    }
                    //If in last byte, go to stage Interest16
                    else {
                        self.stage = Stage::Interest16 {
                            offset: 0,
                            value: 0,
                            endianess: true,
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("interest8 was called while not in a Interest8 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_interest16(
        &mut self,
        offset: usize,
        value: u8,
        endianess: bool,
        datalen: usize,
        effector: bool,
    ) {
        match self.stage {
            //Increase state
            Stage::Interest16 { .. } => {
                if effector {
                    //If value has to increase
                    if value + 1 < (INTERESTING_16_BIT.len() as u8) {
                        self.stage = Stage::Interest16 {
                            offset,
                            value: value + 1,
                            endianess,
                        };
                    }
                    //If endianess has to change
                    else if endianess {
                        self.stage = Stage::Interest16 {
                            offset,
                            value: 0,
                            endianess: false,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 2 < datalen {
                        self.stage = Stage::Interest16 {
                            offset: offset + 1,
                            value,
                            endianess: true,
                        };
                    }
                    //If stage Interest16 is completed
                    else {
                        self.stage = Stage::Interest32 {
                            offset: 0,
                            value: 0,
                            endianess: true,
                        };
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 2 < datalen {
                        self.stage = Stage::Interest16 {
                            offset: offset + 1,
                            value: 0,
                            endianess: true,
                        };
                    }
                    //If in last byte, go to stage Interest32
                    else {
                        self.stage = Stage::Interest32 {
                            offset: 0,
                            value: 0,
                            endianess: true,
                        };
                    }
                }
            }
            //Invalid state
            _ => panic!("interest16 was called while not in a Interest16 stage"),
        };
    }

    //Helper function to change the state to the next apropiate
    fn next_state_interest32(
        &mut self,
        offset: usize,
        value: u8,
        endianess: bool,
        datalen: usize,
        effector: bool,
    ) {
        match self.stage {
            //Increase state
            Stage::Interest32 { .. } => {
                if effector {
                    //If value has to increase
                    if value + 1 < (INTERESTING_32_BIT.len() as u8) {
                        self.stage = Stage::Interest32 {
                            offset,
                            value: value + 1,
                            endianess,
                        };
                    }
                    //If endianess has to change
                    else if endianess {
                        self.stage = Stage::Interest32 {
                            offset,
                            value: 0,
                            endianess: false,
                        };
                    }
                    //If byte offset has to increase
                    else if offset + 4 < datalen {
                        self.stage = Stage::Interest32 {
                            offset: offset + 1,
                            value,
                            endianess: true,
                        };
                    }
                    //If stage Interest32 is completed
                    else {
                        self.stage = Stage::Finished;
                    }
                } else {
                    //If byte/offset has to increase
                    if offset + 4 < datalen {
                        self.stage = Stage::Interest32 {
                            offset: offset + 1,
                            value: 0,
                            endianess: true,
                        };
                    }
                    //If in last byte, go to stage Interest32
                    else {
                        self.stage = Stage::Finished;
                    }
                }
            }
            //Invalid state
            _ => panic!("interest32 was called while not in a Interest32 stage"),
        };
    }

    //Helper function to read 16 bits from vektor
    fn read16(&mut self, data: &mut Vec<u8>, byteoffset: usize) -> u16 {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        return cursor.read_u16::<BigEndian>().expect("RAND_1681307516");
    }

    //Helper function to write 16 bits from vektor
    fn write16(&mut self, data: &mut Vec<u8>, byteoffset: usize, value: u16) {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        cursor
            .write_u16::<BigEndian>(value)
            .expect("RAND_2575765589");
    }

    //Helper function to read 32 bits from vektor
    fn read32(&mut self, data: &mut Vec<u8>, byteoffset: usize) -> u32 {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        return cursor.read_u32::<BigEndian>().expect("RAND_2617398976");
    }

    //Helper function to write 32 bits from vektor
    fn write32(&mut self, data: &mut Vec<u8>, byteoffset: usize, value: u32) {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        cursor
            .write_u32::<BigEndian>(value)
            .expect("RAND_2115066890");
    }

    //Helper function to read 64 bits from vektor
    fn read64(&mut self, data: &mut Vec<u8>, byteoffset: usize) -> u64 {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        return cursor.read_u64::<BigEndian>().expect("RAND_1569949884");
    }

    //Helper function to write 64 bits from vektor
    fn write64(&mut self, data: &mut Vec<u8>, byteoffset: usize, value: u64) {
        let mut cursor = Cursor::new(&mut data[..]);
        cursor.set_position((byteoffset) as u64);
        match cursor.write_u64::<BigEndian>(value) {
            Ok(..) => {}
            Err(e) => panic!(e),
        };
    }

    //Flip 1 bit according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip1(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip1(offset, data.len(), false);
            return ReturnValue::Skip;
        }

        //Flip bit
        //If offset is too large do nothing
        if offset >> 3 < data.len() {
            data[offset >> 3] ^= 128 >> (offset & 7);
        }

        //Calculate new state
        self.next_state_flip1(offset, data.len(), true);

        //Return restore information
        //One bit was changed
        return ReturnValue::ChangedBits {
            range: offset / 8..1 + offset / 8,
        };
    }

    //Flip 2 bits according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip2(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip2(offset, data.len(), false);
            return ReturnValue::Skip;
        }
        //Add dummy byte for Cursor reading and writing
        data.push(0);
        //Read 16bits
        let val: u16 = self.read16(data, offset / 8);
        //Flip 2 bits
        let mask = 0b1100000000000000 >> (offset % 8);
        //Write changes back
        self.write16(data, offset / 8, val ^ mask);
        //delete dummy byte
        data.pop();

        //Calculate new state
        self.next_state_flip2(offset, data.len(), true);

        //Return restore information
        //Two bits were changed
        return ReturnValue::ChangedBits {
            range: offset / 8..1 + ((offset + 1) / 8),
        };
    }

    //Flip 4 bits according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip4(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip4(offset, data.len(), false);
            return ReturnValue::Skip;
        }
        //Add dummy byte for Cursor reading and writing
        data.push(0);
        //Read 16bits
        let val: u16 = self.read16(data, offset / 8);
        //Flip 4 bits
        let mask = 0b1111000000000000 >> (offset % 8);
        //Write changes back
        self.write16(data, offset / 8, val ^ mask);
        //delete dummy byte
        data.pop();

        //Calculate new state
        self.next_state_flip4(offset, data.len(), true);

        //Return restore information
        //Four bits were changed
        return ReturnValue::ChangedBits {
            range: (offset / 8..1 + (offset + 3) / 8),
        };
    }

    //Flip 8 bits according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip8(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip8(offset, data.len(), false);
            return ReturnValue::Skip;
        }
        //Add dummy byte for Cursor reading and writing
        data.push(0);
        //Read 16bits
        let val: u16 = self.read16(data, offset / 8);
        //Flip 8 bits
        let mask = 0b1111111100000000 >> (offset % 8);
        //Write changes back
        self.write16(data, offset / 8, val ^ mask);
        //delete dummy byte
        data.pop();

        //Calculate new state
        self.next_state_flip8(offset, data.len(), true);

        //Return restore information
        //Eight bits were changed
        return ReturnValue::ChangedBits {
            range: (offset / 8..1 + (offset + 7) / 8),
        };
    }

    //Flip 16 bits according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip16(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip16(offset, data.len(), false);
            return ReturnValue::Skip;
        }
        //Add 2 dummy bytes for Cursor reading and writing
        data.push(0);
        data.push(0);
        //Read 32 bits
        let val: u32 = self.read32(data, offset / 8);
        //Flip 16 bits
        let mask = 0b11111111111111110000000000000000 >> (offset % 8);
        //Write changes back
        self.write32(data, offset / 8, val ^ mask);
        //delete dummy bytes
        data.pop();
        data.pop();

        //Calculate new state
        self.next_state_flip16(offset, data.len(), true);

        //Return restore information
        //sixteen bits were changed
        return ReturnValue::ChangedBits {
            range: (offset / 8..1 + (offset + 15) / 8),
        };
    }

    //Flip 32 bits according to the offset
    //This will also change the state, effectively increasing it by one
    fn flip32(&mut self, offset: usize, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset / 8] == 0 {
            //Calculate new state
            self.next_state_flip32(offset, data.len(), false);
            return ReturnValue::Skip;
        }
        //Add 4 dummy bytes for Cursor reading and writing
        data.push(0);
        data.push(0);
        data.push(0);
        data.push(0);
        //Read 32 bits
        let val: u64 = self.read64(data, offset / 8);
        //Flip 32 bits
        let mask =
            0b1111111111111111111111111111111100000000000000000000000000000000 >> (offset % 8);
        //Write changes back
        self.write64(data, offset / 8, val ^ mask);
        //delete dummy bytes
        data.pop();
        data.pop();
        data.pop();
        data.pop();

        //Calculate new state
        self.next_state_flip32(offset, data.len(), true);

        //Return restore information
        //Thirtytwo bits were changed
        return ReturnValue::ChangedBits {
            range: (offset / 8..1 + (offset + 31) / 8),
        };
    }

    //Add or subtract a value between 1 an ARITH_MAX according to the state
    //This will also change the state, effectively increasing it by one
    fn arith8(&mut self, offset: usize, value: i8, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 || value == 0 {
            //Calculate new state
            self.next_state_arith8(offset, value, data.len(), false);
            return ReturnValue::Skip;
        }

        //Calculate which bits would change
        let changed_bits = data[offset] ^ data[offset].wrapping_add(value as u8);

        //Check if already done by a bitflip
        if is_not_bitflip!(change => changed_bits) {
            //Add value
            data[offset] = data[offset].wrapping_add(value as u8);

            //Calculate new state
            self.next_state_arith8(offset, value, data.len(), true);

            //Return which bytes changed
            return ReturnValue::ChangedBits {
                range: (offset..1 + offset),
            };
        }

        //If is bitflip or value==0 go to next state and perform the next state
        self.next_state_arith8(offset, value, data.len(), true);
        return ReturnValue::Skip;
    }

    //Add or substract a value between 1 an ARITH_MAX according to the state
    //On every byte there need to be performed 4*AFL_ARITH_MAX steps because little and big endianess is used
    //Only on last byte there are no steps performed because 2 bytes are needed
    //The last bit of offset determines endianess
    //This function will also change the state, effectively increasing it by one
    fn arith16(
        &mut self,
        offset: usize,
        value: i16,
        endianess: bool,
        data: &mut Vec<u8>,
    ) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 {
            //Calculate new state
            self.next_state_arith16(offset, value, endianess, data.len(), false);
            return ReturnValue::Skip;
        } else if value != 0 {
            //Combine two 8 bit number to one 16 bit number.
            //data is borrowed
            let mut word_value: u16;
            {
                let mut reader = Cursor::new(&mut data[..]);
                reader.set_position(offset as u64);
                if endianess {
                    word_value = reader.read_u16::<BigEndian>().expect("RAND_3668990801");
                } else {
                    word_value = reader.read_u16::<LittleEndian>().expect("RAND_147480249");
                }
            }

            //Calculate which bits would change
            let mut changed_bits: u16 = word_value ^ word_value.wrapping_add(value as u16);
            //If in big endian phase swap changed_bits bytes
            if endianess {
                changed_bits = ((changed_bits & 0xFF) * 0x100) + (changed_bits >> 8);
            }

            //Check if already done by a bitflip or arith8
            if is_not_bitflip_two_bytes!(change => changed_bits)
            	//If value>=0 check if sum is bigger than 0xff
            	&& ((value < 0) || (((word_value & 0xFF)+(value as u16)) > 0xff))
            	//If vaule<0 check if value is bigger than the last byte 
            	&& ((value >= 0) || ((word_value as u8) < (value.abs() as u8)))
            {
                //Add value
                word_value = word_value.wrapping_add(value as u16);

                //Write changes to data
                //data is borrowed
                {
                    let mut writer = Cursor::new(&mut data[..]);
                    writer.set_position(offset as u64);
                    if endianess {
                        writer
                            .write_u16::<BigEndian>(word_value)
                            .expect("RAND_4088602530");
                    } else {
                        writer
                            .write_u16::<LittleEndian>(word_value)
                            .expect("RAND_1045175638");
                    }
                }
                //Calculate new state
                self.next_state_arith16(offset, value, endianess, data.len(), true);

                //Return which bytes changed
                return ReturnValue::ChangedBits {
                    range: (offset..1 + offset + 1),
                };
            }
        }
        //If is bitflip or arith8, value=0 go to next state and perform the next step
        //Calculate new state
        self.next_state_arith16(offset, value, endianess, data.len(), true);
        return ReturnValue::Skip;
    }

    //Add or substract a value between 1 an ARITH_MAX according to the state
    //On every byte there need to be performed 4*AFL_ARITH_MAX steps because little and big endianess is used
    //Only on the last three bytes there are no steps performed because 4 bytes are needed
    //The last bit of offset determines if addition or subtraction
    //The second last bit of offset determinse endianess
    //This will also change the state, effectively increasing it by one
    fn arith32(
        &mut self,
        offset: usize,
        value: i32,
        endianess: bool,
        data: &mut Vec<u8>,
    ) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 {
            //Calculate new state
            self.next_state_arith32(offset, value, endianess, data.len(), false);
            return ReturnValue::Skip;
        } else if value != 0 {
            //Combine two 8 bit number to one 32 bit number.
            //data is borrowed
            let mut double_word_value: u32;
            {
                let mut reader = Cursor::new(&mut data[..]);
                reader.set_position(offset as u64);
                if endianess {
                    double_word_value = reader.read_u32::<BigEndian>().expect("RAND_666587179");
                } else {
                    double_word_value = reader.read_u32::<LittleEndian>().expect("RAND_166277171");
                }
            }

            //Calculate which bits would change
            let mut changed_bits: u32 =
                double_word_value ^ double_word_value.wrapping_add(value as u32);
            //If in big endian phase swap changed_bits bytes
            if endianess {
                changed_bits = ((changed_bits & 0xFF) * 0x1000000)
                    + (((changed_bits >> 8) & 0xFF) * 0x10000)
                    + (((changed_bits >> 16) & 0xFF) * 0x100)
                    + (changed_bits >> 24);
            }

            //Check if already done by a bitflip or arith8
            if is_not_bitflip_two_bytes!(change => changed_bits)
                && ((value < 0) || (((double_word_value & 0xFFFF) + (value as u32)) > 0xFFFF))
                && ((value >= 0) || ((double_word_value as u16) < (value.abs() as u16)))
            {
                //Add value
                double_word_value = double_word_value.wrapping_add(value as u32);

                //Write changes to data
                //data is borrowed
                {
                    let mut writer = Cursor::new(&mut data[..]);
                    writer.set_position(offset as u64);
                    if endianess {
                        writer
                            .write_u32::<BigEndian>(double_word_value)
                            .expect("RAND_3444404423");
                    } else {
                        writer
                            .write_u32::<LittleEndian>(double_word_value)
                            .expect("RAND_4282527583");
                    }
                }
                //Calculate new state
                self.next_state_arith32(offset, value, endianess, data.len(), true);

                //Return which bytes changed
                return ReturnValue::ChangedBits {
                    range: (offset..1 + offset + 3),
                };
            }
        }
        //If is bitflip, arith8/16, or value=0 go to next state and perform the next step
        self.next_state_arith32(offset, value, endianess, data.len(), true);
        return ReturnValue::Skip;
    }

    //Overwrite current byte with an interesting value from the INTERESTING_8_BIT array
    //Also increases the offset of the state
    //9 steps per byte
    fn interest8(&mut self, offset: usize, value: u8, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 {
            //Calculate new state
            self.next_state_interest8(offset, value, data.len(), false);
            return ReturnValue::Skip;
        }
        //Overwrite value
        data[offset] = INTERESTING_8_BIT[value as usize];
        //Calculate new state
        self.next_state_interest8(offset, value, data.len(), true);
        return ReturnValue::ChangedBits {
            range: (offset..1 + offset),
        };
    }

    //Overwrite current byte with an interesting value from the INTERESTING_16_BIT array
    //Also increases the offset of the state
    //20 steps per byte. 10 value and 2 steps for each value because of endianess
    //Last bit of offset defines endianess
    fn interest16(
        &mut self,
        offset: usize,
        value: u8,
        endianess: bool,
        data: &mut Vec<u8>,
    ) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 {
            //Calculate new state
            self.next_state_interest16(offset, value, endianess, data.len(), false);
            return ReturnValue::Skip;
        } else {
            //Overwrite value
            //data is borrowed
            {
                let mut writer = Cursor::new(&mut data[..]);
                writer.set_position(offset as u64);
                if endianess {
                    writer
                        .write_u16::<BigEndian>(INTERESTING_16_BIT[value as usize])
                        .expect("RAND_1790163083");
                } else {
                    writer
                        .write_u16::<LittleEndian>(INTERESTING_16_BIT[value as usize])
                        .expect("RAND_3961852076");
                }
            }

            //Calculate new state
            self.next_state_interest16(offset, value, endianess, data.len(), true);
            return ReturnValue::ChangedBits {
                range: (offset..1 + offset + 1),
            };
        }
    }

    //Overwrite current four bytes with an interesting value from the INTERESTING_32_BIT array
    //Also increases the offset of the state
    //16 steps per byte. 8 values and 2 steps for each value because of endianess
    //Last bit of offset defines endianess
    fn interest32(
        &mut self,
        offset: usize,
        value: u8,
        endianess: bool,
        data: &mut Vec<u8>,
    ) -> ReturnValue {
        //If effector map is 0 for the current byte go to next byte and continue there
        if self.effector_map[offset] == 0 {
            //Calculate new state
            self.next_state_interest32(offset, value, endianess, data.len(), false);
            return ReturnValue::Skip;
        } else {
            //Overwrite value
            //data is borrowed
            {
                let mut writer = Cursor::new(&mut data[..]);
                writer.set_position(offset as u64);
                if endianess {
                    writer
                        .write_u32::<BigEndian>(INTERESTING_32_BIT[value as usize])
                        .expect("RAND_1912486925");
                } else {
                    writer
                        .write_u32::<LittleEndian>(INTERESTING_32_BIT[value as usize])
                        .expect("RAND_399438168");
                }
            }

            //Calculate new state
            self.next_state_interest32(offset, value, endianess, data.len(), true);
            return ReturnValue::ChangedBits {
                range: (offset..1 + offset + 3),
            };
        }
    }

    fn finished(&mut self) -> ReturnValue {
        //TODO
        ReturnValue::Finished
    }

    fn bruteforce(&mut self, offset: usize, value: u8, data: &mut Vec<u8>) -> ReturnValue {
        //If effector map is 0 for the current byte start deterministic from the beginning
        if self.effector_map[offset] == 0 {
            self.stage = Stage::Flip1 { offset: 0 };
            return ReturnValue::Skip;
        }
        //Try next value
        data[offset] = value;

        //Calculate next state
        if value < 0xFF {
            self.stage = Stage::Bruteforce {
                offset,
                value: value + 1,
            };
        } else {
            self.stage = Stage::Flip1 { offset: 0 };
        }
        return ReturnValue::ChangedBits {
            range: (offset..1 + offset),
        };
    }

    //Functions for havoc

    fn set_random_byte(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        data[rng.gen_range(0, datalen)] = rng.gen_range(0, 255);
    }
    fn flip_random(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen * 8);
        //Flip bit
        data[offset >> 3] ^= 128 >> (offset & 7);
    }
    fn set_random_interest8(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen);
        let value = rng.gen_range(0, INTERESTING_8_BIT.len() as u8);
        //Overwrite value
        data[offset] = INTERESTING_8_BIT[value as usize];
    }
    fn set_random_interest16(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 1);
        let value = rng.gen_range(0, INTERESTING_16_BIT.len() as u8);
        let endianess = rng.gen();
        //Overwrite value
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u16::<BigEndian>(INTERESTING_16_BIT[value as usize])
                    .expect("RAND_1875432873");
            } else {
                writer
                    .write_u16::<LittleEndian>(INTERESTING_16_BIT[value as usize])
                    .expect("RAND_319618939");
            }
        }
    }
    fn set_random_interest32(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 3);
        let value = rng.gen_range(0, INTERESTING_32_BIT.len() as u8);
        let endianess = rng.gen();
        //Overwrite value
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u32::<BigEndian>(INTERESTING_32_BIT[value as usize])
                    .expect("RAND_1327107930");
            } else {
                writer
                    .write_u32::<LittleEndian>(INTERESTING_32_BIT[value as usize])
                    .expect("RAND_1305800536");
            }
        }
    }
    fn subtract_random_arith8(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen);
        let value = rng.gen_range(-(AFL_ARITH_MAX as i8), -1);
        //Add value
        data[offset] = data[offset].wrapping_add(value as u8);
    }
    fn add_random_arith8(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen);
        let value = rng.gen_range(1, AFL_ARITH_MAX as u8);
        //Add value
        data[offset] = data[offset].wrapping_add(value as u8);
    }
    fn subtract_random_arith16(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 1);
        let value = rng.gen_range(-(AFL_ARITH_MAX as i16), -1);
        let endianess = rng.gen();

        //Combine two 8 bit number to one 16 bit number.
        //data is borrowed
        let mut word_value: u16;
        {
            let mut reader = Cursor::new(&mut data[..]);
            reader.set_position(offset as u64);
            if endianess {
                word_value = reader.read_u16::<BigEndian>().expect("RAND_2855956511");
            } else {
                word_value = reader.read_u16::<LittleEndian>().expect("RAND_3624264746");
            }
        }

        //Add value
        word_value = word_value.wrapping_add(value as u16);

        //Write changes to data
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u16::<BigEndian>(word_value)
                    .expect("RAND_3506660419");
            } else {
                writer
                    .write_u16::<LittleEndian>(word_value)
                    .expect("RAND_3070521794");
            }
        }
    }
    fn add_random_arith16(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 1);
        let value = rng.gen_range(1, AFL_ARITH_MAX as i16);
        let endianess = rng.gen();
        //Combine two 8 bit number to one 16 bit number.
        //data is borrowed
        let mut word_value: u16;
        {
            let mut reader = Cursor::new(&mut data[..]);
            reader.set_position(offset as u64);
            if endianess {
                word_value = reader.read_u16::<BigEndian>().expect("RAND_1628808888");
            } else {
                word_value = reader.read_u16::<LittleEndian>().expect("RAND_3892433555");
            }
        }

        //Add value
        word_value = word_value.wrapping_add(value as u16);

        //Write changes to data
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u16::<BigEndian>(word_value)
                    .expect("RAND_2941915438");
            } else {
                writer
                    .write_u16::<LittleEndian>(word_value)
                    .expect("RAND_2971751543");
            }
        }
    }
    fn subtract_random_arith32(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 3);
        let value = rng.gen_range(-(AFL_ARITH_MAX as i32), -1);
        let endianess = rng.gen();
        //Combine two 8 bit number to one 32 bit number.
        //data is borrowed
        let mut double_word_value: u32;
        {
            let mut reader = Cursor::new(&mut data[..]);
            reader.set_position(offset as u64);
            if endianess {
                double_word_value = reader.read_u32::<BigEndian>().expect("RAND_1697523139");
            } else {
                double_word_value = reader.read_u32::<LittleEndian>().expect("RAND_2621262421");
            }
        }

        //Add value
        double_word_value = double_word_value.wrapping_add(value as u32);

        //Write changes to data
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u32::<BigEndian>(double_word_value)
                    .expect("RAND_754824763");
            } else {
                writer
                    .write_u32::<LittleEndian>(double_word_value)
                    .expect("RAND_2731291701");
            }
        }
    }
    fn add_random_arith32(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 3);
        let value = rng.gen_range(1, AFL_ARITH_MAX as i32);
        let endianess = rng.gen();
        //Combine two 8 bit number to one 32 bit number.
        //data is borrowed
        let mut double_word_value: u32;
        {
            let mut reader = Cursor::new(&mut data[..]);
            reader.set_position(offset as u64);
            if endianess {
                double_word_value = reader.read_u32::<BigEndian>().expect("RAND_119495844");
            } else {
                double_word_value = reader.read_u32::<LittleEndian>().expect("RAND_1626853211");
            }
        }

        //Add value
        double_word_value = double_word_value.wrapping_add(value as u32);

        //Write changes to data
        //data is borrowed
        {
            let mut writer = Cursor::new(&mut data[..]);
            writer.set_position(offset as u64);
            if endianess {
                writer
                    .write_u32::<BigEndian>(double_word_value)
                    .expect("RAND_973706992");
            } else {
                writer
                    .write_u32::<LittleEndian>(double_word_value)
                    .expect("RAND_892448454");
            }
        }
    }
    fn delete_random_bytes(&mut self, data: &mut Vec<u8>, datalen: usize) {
        if datalen > 4 {
            let mut rng = thread_rng();
            let offset = rng.gen_range(0, datalen - 1);
            let delete_len = rng.gen_range(
                0,
                cmp::min(AFL_HAVOC_BLK_LARGE, cmp::min(datalen - offset, datalen - 3)),
            );
            for _ in 0..delete_len {
                data.remove(offset);
            }
        }
    }
    fn insert_random_bytes(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 1);
        let insert_len = rng.gen_range(1, cmp::min(AFL_HAVOC_BLK_LARGE, datalen - offset));
        let not_clone = rng.gen_weighted_bool(4);
        if not_clone {
            let random_value = rng.gen_range(0, 255);
            for x in offset..(offset + insert_len) {
                data.insert(x, random_value);
            }
        } else {
            let clone_from = rng.gen_range(0, datalen - insert_len);
            if clone_from > offset {
                for x in 0..insert_len {
                    let byte_to_clone = data[clone_from + x + x]; //Plus two times x because we insert before the bytes to clone
                    data.insert(offset + x, byte_to_clone);
                }
            } else {
                for x in 0..insert_len {
                    let byte_to_clone = data[clone_from + x];
                    data.insert(offset + x, byte_to_clone);
                }
            }
        }
    }
    fn overwrite_random_bytes(&mut self, data: &mut Vec<u8>, datalen: usize) {
        let mut rng = thread_rng();
        let offset = rng.gen_range(0, datalen - 1);
        let overwrite_len = rng.gen_range(1, cmp::min(AFL_HAVOC_BLK_LARGE, datalen - offset));
        let not_clone = rng.gen_weighted_bool(4);
        if not_clone {
            let random_value = rng.gen_range(0, 255);
            for x in offset..(offset + overwrite_len) {
                data[x] = random_value;
            }
        } else {
            //Problem: clone_from slightly smaller than offset
            let clone_from = rng.gen_range(0, datalen - overwrite_len);
            for x in 0..overwrite_len {
                let byte_to_clone = data[clone_from + x];
                data[offset + x] = byte_to_clone;
            }
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    //Test for Flip1
    #[test]
    fn check_return_value_flip1() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..1));
    }
    #[test]
    fn check_stage_changes1_flip1() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip1 { offset } => assert_eq!(8, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_flip1() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 0 },
            effector_map: vec![0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip1 { offset } => assert_eq!(9, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes3_flip1() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 7 },
            effector_map: vec![0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip2 { offset } => assert_eq!(0, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes4_flip1() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 8 },
            effector_map: vec![1, 0],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip2 { offset } => assert_eq!(1, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_flip1() {
        let mut v = vec![0b00001111u8, 0b01010101u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip1 { offset: 1 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b01001111u8);
    }

    //Tests for Flip2
    #[test]
    fn check_return_value_flip2() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..1));
    }
    #[test]
    fn check_stage_changes1_flip2() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip2 { offset } => assert_eq!(8, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_flip2() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 0 },
            effector_map: vec![0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip2 { offset } => assert_eq!(9, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes3_flip2() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 6 },
            effector_map: vec![0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip4 { offset } => assert_eq!(0, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes4_flip2() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 8 },
            effector_map: vec![1, 0],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip4 { offset } => assert_eq!(1, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_flip2() {
        let mut v = vec![0b00001111u8, 0b01010101u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip2 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001110u8);
        assert_eq!(v[1], 0b11010101u8);
    }

    //Tests for Flip4
    #[test]
    fn check_return_value_flip4() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..1));
    }
    #[test]
    fn check_stage_changes1_flip4() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip4 { offset } => assert_eq!(8, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_flip4() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 0 },
            effector_map: vec![0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip4 { offset } => assert_eq!(9, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes3_flip4() {
        let mut v = vec![12, 13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 12 },
            effector_map: vec![1, 0, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip8 { offset } => assert_eq!(0, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes4_flip4() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 8 },
            effector_map: vec![1, 0],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip8 { offset } => assert_eq!(1, (offset)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_flip4() {
        let mut v = vec![0b00001111u8, 0b01010101u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip4 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001110u8);
        assert_eq!(v[1], 0b10110101u8);
    }

    //Tests for Flip8
    #[test]
    fn check_return_value_flip8() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip8 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..2));
    }
    #[test]
    fn check_stage_changes_flip8() {
        let mut v = vec![13, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip8 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip8 { offset } => assert_eq!(8, offset),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_flip8() {
        let mut v = vec![0b00001111u8, 0b01010101u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip8 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001110u8);
        assert_eq!(v[1], 0b10101011u8);
    }

    //Tests for Flip16
    #[test]
    fn check_return_value_flip16() {
        let mut v = vec![13, 12, 14];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip16 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..3));
    }
    #[test]
    fn check_stage_changes_flip16() {
        let mut v = vec![13, 12, 20];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip16 { offset: 8 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip32 { offset } => assert_eq!(0, offset),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes1_flip16() {
        let mut v = vec![0b00001111u8, 0b01010101u8, 0b11111111u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip16 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001110u8);
        assert_eq!(v[1], 0b10101010u8);
        assert_eq!(v[2], 0b00000001u8);
    }
    #[test]
    fn check_data_changes2_flip16() {
        let mut v = vec![0b00001111u8, 0b01010101u8, 0b11111111u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip16 { offset: 8 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001111u8);
        assert_eq!(v[1], 0b10101010u8);
        assert_eq!(v[2], 0b00000000u8);
    }

    //Tests for Flip32
    #[test]
    fn check_return_value_flip32() {
        let mut v = vec![13, 12, 11, 10, 9];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip32 { offset: 4 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..5));
    }
    #[test]
    fn check_stage_changes1_flip32() {
        let mut v = vec![13, 12, 12, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip32 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip32 { offset } => assert_eq!(8, offset),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_flip32() {
        let mut v = vec![13, 12, 12, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip32 { offset: 8 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith8 { offset, value } => assert_eq!((0, -35), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes1_flip32() {
        let mut v = vec![0b00001111u8, 0b01010101u8, 0b00000000u8, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip32 { offset: 7 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001110u8);
        assert_eq!(v[1], 0b10101010u8);
    }
    #[test]
    fn check_data_changes2_flip32() {
        let mut v = vec![0b00001111u8, 0b01010101u8, 0b00000000u8, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Flip32 { offset: 0 },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b11110000u8);
        assert_eq!(v[1], 0b10101010u8);
        assert_eq!(v[2], 0b11111111u8);
    }

    //Tests for arith8
    #[test]
    fn check_return_value_arith8() {
        let mut v = vec![0b00001100, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith8 {
                offset: 0,
                value: -1,
            },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001011u8);
        assert_eq!(x, Some(0..1));
    }
    #[test]
    fn check_stage_changes1_arith8() {
        let mut v = vec![13, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith8 {
                offset: 0,
                value: -35,
            },
            effector_map: vec![0, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith8 { offset, value } => assert_eq!((1, -34), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_arith8() {
        let mut v = vec![12, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith8 {
                offset: 0,
                value: -1,
            },
            effector_map: vec![1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith8 { offset, value } => assert_eq!((0, 0), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_arith8() {
        let mut v = vec![0b00001001u8, 0b01010101u8];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith8 {
                offset: 0,
                value: -2,
            },
            effector_map: vec![1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00000111u8);
        assert_eq!(v[1], 0b01010101u8);
    }

    //Tests for arith16
    #[test]
    fn check_return_value_arith16() {
        let mut v = vec![0b00001100, 0b00001100, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith16 {
                offset: 0,
                value: -13,
                endianess: true,
            },
            effector_map: vec![1, 1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..2));
    }
    #[test]
    fn check_stage_changes1_arith16() {
        let mut v = vec![0b00000000, 0b01000000, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith16 {
                offset: 0,
                value: -12,
                endianess: false,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith16 { offset, value, .. } => assert_eq!((0, -11), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_arith16() {
        let mut v = vec![12, 0b00000000, 0b01000000, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith16 {
                offset: 0,
                value: 0,
                endianess: true,
            },
            effector_map: vec![0, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith16 { offset, value, .. } => assert_eq!((1, -34), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_arith16() {
        let mut v = vec![0b00001000u8, 0b00000000u8, 12, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith16 {
                offset: 0,
                value: -1,
                endianess: true,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00000111u8);
        assert_eq!(v[1], 0b11111111u8);
    }

    //Tests for arith32
    #[test]
    fn check_return_value_arith32() {
        let mut v = vec![0b00001100, 0b00001100, 0, 0, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -13,
                endianess: true,
            },
            effector_map: vec![1, 1, 1],
            options: Options { change_size: false },
        };
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(0..4));
    }
    #[test]
    fn check_stage_changes1_arith32() {
        let mut v = vec![0, 0, 0, 0b01000000, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -12,
                endianess: false,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith32 { offset, value, .. } => assert_eq!((0, -11), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_arith32() {
        let mut v = vec![12, 0b00000000, 0b01000000, 0, 0];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -1,
                endianess: true,
            },
            effector_map: vec![0, 1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Arith32 { offset, value, .. } => assert_eq!((1, -34), (offset, value)),
            Stage::Interest8 { offset, value } => {
                assert_eq!(2, offset);
                assert_eq!(0, value)
            }
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_arith32() {
        let mut v = vec![0b00001000u8, 0b00000000u8, 0, 0];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -1,
                endianess: true,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00000111u8);
        assert_eq!(v[1], 0b11111111u8);
        assert_eq!(v[2], 0b11111111u8);
        assert_eq!(v[3], 0b11111111u8);
    }

    //Tests for bruteforce
    #[test]
    fn check_return_value_bruteforce() {
        let mut v = vec![0b00001100, 0b00001100, 0, 0, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -13,
                endianess: true,
            },
            effector_map: vec![1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.startbruteforce(1);
        let x = my_mut_state.deterministic(&mut v);
        assert_eq!(x, Some(1..2));
    }
    #[test]
    fn check_stage_changes1_bruteforce() {
        let mut v = vec![0, 0, 0, 0b01000000, 12];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -12,
                endianess: false,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.startbruteforce(0);
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Bruteforce { offset, value } => assert_eq!((0, 1), (offset, value)),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes2_bruteforce() {
        let mut v = vec![12, 0b00000000, 0b01000000, 0, 0];
        let mut my_mut_state = MutationState {
            stage: Stage::Bruteforce {
                offset: 1,
                value: 255,
            },
            effector_map: vec![0, 1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip1 { offset } => assert_eq!(0, offset),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_stage_changes3_bruteforce() {
        let mut v = vec![12, 0b00000000, 0b01000000, 0, 0];
        let mut my_mut_state = MutationState {
            stage: Stage::Arith32 {
                offset: 0,
                value: -12,
                endianess: false,
            },
            effector_map: vec![1, 0, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.startbruteforce(1);
        my_mut_state.deterministic(&mut v);
        match my_mut_state.stage {
            Stage::Flip1 { offset } => assert_eq!(1, offset),
            _ => panic!("Wrong stage"),
        };
    }
    #[test]
    fn check_data_changes_bruteforce() {
        let mut v = vec![0b00001000u8, 0b00000000u8, 0, 0];
        let mut my_mut_state = MutationState {
            stage: Stage::Bruteforce {
                offset: 3,
                value: 123,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b00001000u8);
        assert_eq!(v[1], 0b00000000u8);
        assert_eq!(v[2], 0b00000000u8);
        assert_eq!(v[3], 123);
    }

    //Test Havoc function
    #[test]
    fn test_havoc() {
        let mut my_mut_state = MutationState {
            stage: Stage::Bruteforce {
                offset: 0,
                value: 0,
            },
            effector_map: vec![1, 1, 1, 1, 1],
            options: Options { change_size: true },
        };
        //my_mut_state.start_havoc();
        //let mut changed_bits: Range<usize> = 0..0;
        for _ in 0..10000 {
            let mut v = vec![
                0b00001111u8,
                0b00001111u8,
                0b11111111u8,
                0b11111100u8,
                0b11110000u8,
            ];
            my_mut_state.havoc(&mut v);
            //print!("v_len: {}, v: ", v.len());
            //for x in 0..v.len() {
            //    print!("{} ", v[x]);
            //}
            //print!("\n");
        }
    }
    //Test Constructor
    #[test]
    fn test_constructor_bitflip() {
        let mut my_mut_state = MutationState::new_bitflip(vec![1, 1, 1, 1]);
        let mut v = vec![0b00001000u8, 0b00000000u8, 0, 0];
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b10001000u8);
        assert_eq!(v[1], 0b00000000u8);
        assert_eq!(v[2], 0b00000000u8);
        assert_eq!(v[3], 0b00000000u8);
    }
    //Test Constructor
    #[test]
    fn test_constructor_deterministic() {
        let mut my_mut_state = MutationState::new_deterministic(vec![1, 1, 1, 1]);
        let mut v = vec![0b00001000u8, 0b00000000u8, 0, 0];
        my_mut_state.deterministic(&mut v);
        assert_eq!(v[0], 0b11100101u8);
        assert_eq!(v[1], 0b00000000u8);
        assert_eq!(v[2], 0b00000000u8);
        assert_eq!(v[3], 0b00000000u8);
    }
    //Test deterministic_flip_bits function
    #[test]
    fn test_deterministic_flip_bits() {
        let mut my_mut_state = MutationState::new_bitflip(vec![1, 1, 1, 1]);
        //Do-While
        while {
            let mut v = vec![0b00001111u8, 0b00001111u8, 0b11111111u8, 0b11111100u8];
            None != my_mut_state.deterministic_flip_bits(&mut v)
            //println!("v = ({:08b}, {:08b}, {:08b}, {:08b}), changed_bits: {}..{}", v[0], v[1], v[2], v[3], changed_bits.start, changed_bits.end);
        } {}
        match my_mut_state.stage {
            Stage::Arith8 { offset, value } => {
                assert_eq!((0, -(AFL_ARITH_MAX as i8)), (offset, value))
            }
            _ => panic!("Wrong stage"),
        };
    }
    //Test deterministic_flip_bits and deterministic together
    #[test]
    fn test_deterministic_flip_bits_and_deterministic() {
        let mut my_mut_state = MutationState::new_bitflip(vec![1, 1, 1, 1]);
        //Do-While
        while {
            let mut v = vec![0b00001111u8, 0b00001111u8, 0b11111111u8, 0b11111100u8];
            None != my_mut_state.deterministic_flip_bits(&mut v)
            //println!("v = ({:08b}, {:08b}, {:08b}, {:08b}), changed_bits: {}..{}", v[0], v[1], v[2], v[3], changed_bits.start, changed_bits.end);
        } {}
        match my_mut_state.stage {
            Stage::Arith8 { offset, value } => {
                assert_eq!((0, -(AFL_ARITH_MAX as i8)), (offset, value))
            }
            _ => panic!("Wrong stage"),
        };
        //Do-While
        while {
            let mut v = vec![0b00001111u8, 0b00001111u8, 0b11111111u8, 0b11111100u8];
            None != my_mut_state.deterministic(&mut v)
            //println!("v = ({:08b}, {:08b}, {:08b}, {:08b}), changed_bits: {}..{}", v[0], v[1], v[2], v[3], changed_bits.start, changed_bits.end);
        } {}
        match my_mut_state.stage {
            Stage::Finished => (),
            _ => panic!("Wrong stage"),
        };
    }

    //Final test run through all stages
    #[test]
    fn run() {
        let mut my_mut_state = MutationState {
            stage: Stage::Bruteforce {
                offset: 0,
                value: 0,
            },
            effector_map: vec![1, 1, 1, 1],
            options: Options { change_size: false },
        };
        //Do-While
        while {
            let mut v = vec![0b00001111u8, 0b00001111u8, 0b11111111u8, 0b11111100u8];
            None != my_mut_state.deterministic(&mut v)
            //println!("v = ({:08b}, {:08b}, {:08b}, {:08b}), changed_bits: {}..{}", v[0], v[1], v[2], v[3], changed_bits.start, changed_bits.end);
        } {}
        match my_mut_state.stage {
            Stage::Finished => (),
            _ => panic!("Wrong stage"),
        };
    }
}
