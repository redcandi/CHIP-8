use rand::random;
use crate::font;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const REG_LENGTH: usize = 16;
const STACK_SIZE: usize = 16;
const KEY_LENGTH: usize = 16;

const START_POS: u16 = 0x200;

pub struct Processor {
    program_counter: u16,
    ram: [u8; RAM_SIZE],
    vram: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_registers: [u8; REG_LENGTH],
    ram_index: u16,
    stack: [u16; STACK_SIZE], //to remember where we jumped from during a subroutine!
    stack_pointer: u16,
    keys: [bool; KEY_LENGTH],
    delay_timer: u8,
    sound_timer: u8,
}

impl Processor {
    pub fn new() -> Self {
        let mut ram = [0u8; RAM_SIZE];

        ram[..font::FONT_SIZE].copy_from_slice(&font::FONTSET);

        Self {
            program_counter: START_POS,
            ram: ram,
            vram: [false; SCREEN_HEIGHT * SCREEN_WIDTH],
            v_registers: [0; REG_LENGTH],
            ram_index: 0,
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            keys: [false; KEY_LENGTH],
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    pub fn get_display(&self) -> &[bool]{
        &self.vram
    }

    pub fn key_input(&mut self, idx : usize, pressed: bool){
        self.keys[idx] = pressed;
    }

    pub fn load_state(&mut self, data : &[u8]){
        let start = START_POS as usize;
        let end = (START_POS as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    pub fn reset(&mut self) {
        self.program_counter = START_POS;
        self.ram = [0u8; RAM_SIZE];
        self.vram = [false; SCREEN_HEIGHT * SCREEN_WIDTH];
        self.v_registers = [0; REG_LENGTH];
        self.ram_index = 0;
        self.stack = [0; STACK_SIZE];
        self.stack_pointer = 0;
        self.keys = [false; KEY_LENGTH];
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.ram[..font::FONT_SIZE].copy_from_slice(&font::FONTSET);
    }

    pub fn tick(&mut self) {
        //"Ticks" Once every CPU cycle, the emulated one, not your stupid INTEL slop
        //Fetch
        let op = self.fetch();
        //Decode and Execute
        self.execute(op);
    }

    fn fetch(&mut self) -> u16 {
        //grabs the opcode
        //ram is u8 but opcodes are u16 so it's time for bitmask fuckery
        let most_sig_half = self.ram[self.program_counter as usize] as u16;
        let least_sig_half = self.ram[(self.program_counter + 1) as usize] as u16;
        let op = (most_sig_half << 8) | least_sig_half; //big endian :3 smth i learned
        self.program_counter += 2;
        op
    }

    fn execute(&mut self, op: u16) {
        //convert all to u8 later, too unbothered to do it now
        let digits = (
            (op & 0xF000) >> 12,
            (op & 0x0F00) >> 8,
            (op & 0x00F0) >> 4,
            (op & 0x000F),
        );

        match digits {
            //i wish i had a plugin to put nvim in focus for this specific block lol
            (0, 0, 0, 0) => return, //NOP - 0000

            (0, 0, 0xE, 0) => {
                //CLS - 00E0 : clear the vram buffer
                self.vram = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }

            (0, 0, 0xE, 0xE) => {
                //RET - 00EE : return from subroutine
                self.program_counter = self.pop();
            }

            (0x1, _, _, _) => {
                //JMP NNN - 1NNN : jumps pc to NNN
                self.program_counter = op & 0xFFF;
                //notice how 0xfff defines our 4kb memory space
            }

            (0x2, _, _, _) => {
                //CALL NNN - 2NNN : calls a subroutine
                self.push(self.program_counter);
                self.program_counter = op & 0xFFF;
            }

            (0x3, _, _, _) => {
                //SKIP VX == NN - 3XNN : skips next if VX == NN (VX being Xth V reg.)
                let x = digits.1 as usize;
                if self.v_registers[x] == (op & 0xFF) as u8 {
                    self.program_counter += 2;
                }
            }

            (0x4, _, _, _) => {
                //SKIP VX != NN - 4XNN : same as prev but for ineq.
                let x = digits.1 as usize;
                if self.v_registers[x] != (op & 0xFF) as u8 {
                    self.program_counter += 2;
                }
            }

            (0x5, _, _, 0x0) => {
                //SKIP VX == VY - 5XY0
                if self.v_registers[digits.1 as usize] == self.v_registers[digits.2 as usize] {
                    self.program_counter += 2;
                }
            }

            (0x6, _, _, _) => {
                //VX = VN - 6XNN : Set V register to the NN
                self.v_registers[digits.1 as usize] = (op & 0xFF) as u8;
            }

            (0x7, _, _, _) => {
                //VX += NN - 7XNN : WRAP addition
                self.v_registers[digits.1 as usize] =
                    self.v_registers[digits.1 as usize].wrapping_add((op & 0xFF) as u8);
            }

            (0x8, _, _, 0x0) => {
                // VX = VY - 8XY0
                self.v_registers[digits.1 as usize] = self.v_registers[digits.2 as usize];
            }

            (0x8, _, _, 0x1) => {
                // VX |= VY - 8XY1
                self.v_registers[digits.1 as usize] |= self.v_registers[digits.2 as usize];
            }

            (0x8, _, _, 0x2) => {
                // VX &= VY - 8XY2
                self.v_registers[digits.1 as usize] &= self.v_registers[digits.2 as usize];
            }

            (0x8, _, _, 0x3) => {
                // VX ^= VY - 8XY3
                self.v_registers[digits.1 as usize] ^= self.v_registers[digits.2 as usize];
            }

            (0x8, _, _, 0x4) => {
                //VX += VY - 8XY4 : add that uses the flag register so carry is used
                let x = digits.1 as usize;
                let y = digits.2 as usize;
                let (res, flag) = self.v_registers[x].overflowing_add(self.v_registers[y]);
                self.v_registers[x] = res;
                self.v_registers[0xF] = if flag { 1 } else { 0 };
            }

            (0x8, _, _, 0x5) => {
                //VX -= VY - 8XY5 : sub that uses the flag register so carry is used
                let x = digits.1 as usize;
                let y = digits.2 as usize;
                let (res, flag) = self.v_registers[x].overflowing_sub(self.v_registers[y]);
                self.v_registers[x] = res;
                self.v_registers[0xF] = if flag { 0 } else { 1 };
            }

            (0x8, _, _, 0x6) => {
                //VX >>= 1 - 8XY6 : right shift, flag = dropped bit
                let x = digits.1 as usize;
                let least_sig_bit = self.v_registers[x] & 1u8;
                self.v_registers[x] >>= 1;
                self.v_registers[0xF] = least_sig_bit;
            }

            (0x8, _, _, 0x7) => {
                //VX = (VY - VX) - 8XY7
                let x = digits.1 as usize;
                let y = digits.2 as usize;
                let (res, flag) = self.v_registers[y].overflowing_sub(self.v_registers[x]);
                self.v_registers[x] = res;
                self.v_registers[0xF] = if flag { 0 } else { 1 };
            }

            (0x8, _, _, 0xE) => {
                // VX <<= 1 - 8XYE : left shift
                let x = digits.1 as usize;
                let most_sig_bit = (self.v_registers[x] >> 7) & 1u8;
                self.v_registers[x] <<= 1;
                self.v_registers[0xF] = most_sig_bit;
            }

            (0x9, _, _, 0x0) => {
                //SKIP if VX!= VY - 9XY0
                let x = digits.1 as usize;
                let y = digits.2 as usize;
                if self.v_registers[x] != self.v_registers[y] {
                    self.program_counter += 2;
                }
            }

            (0xA, _, _, _) => {
                // ram_index = NNN - ANNN : first use of ram_index, a setter code
                self.ram_index = op & 0xFFF;
            }

            (0xB, _, _, _) => {
                // JUMP PC = V0 + NNN - BNNN : jumps PC to V0 + NNN
                self.program_counter = (self.v_registers[0] as u16) + (op & 0xFFF);
            }

            (0xC, _, _, _) => {
                // VX = rand() & NN - CXNN : rng with and for some reason?
                let rng: u8 = random();
                self.v_registers[digits.1 as usize] = rng & ((op & 0xFF) as u8);
            }

            (0xD, _, _, _) => {
                // DRAW - DXYN : i'm regretting using 1d-array for vram buffer
                // this is peak function, every single sprite uses 1 byte or 8 bits
                // for width. And they can be N rows long. SOOOO ram_index is used
                // to store sprite location and *ram_index + 1... N-1 has the sprite
                // peak cinema !--OwO--!
                let x_cord = self.v_registers[digits.1 as usize] as u16;
                let y_cord = self.v_registers[digits.2 as usize] as u16;
                let rows = digits.3;
                //check if any pixel was flipped
                let mut flipped = false;
                //Iterating over our sprite rows
                for row in 0..rows {
                    let ptr = self.ram_index + row as u16;
                    let pixels = self.ram[ptr as usize];
                    //the magic is that each pixel value is 8 bit and 1 flips the pxl
                    for col in 0..8 {
                        //bit mask fuckery time
                        if (pixels & (0b1000_0000 >> col)) != 0 {
                            let x = (x_cord + col) as usize % SCREEN_WIDTH;
                            let y = (y_cord + row) as usize % SCREEN_HEIGHT;
                            //access the current value in vram buffer
                            let idx = x + SCREEN_WIDTH * y;
                            flipped |= self.vram[idx];
                            self.vram[idx] ^= true;
                        }
                    }
                }

                if flipped {
                    self.v_registers[0xF] = 1;
                } else {
                    self.v_registers[0xF] = 0;
                }
            }

            (0xE, _, 0x9, 0xE) => {
                // SKIP KEY PRESS - EX9E : skip if key at X is pressed
                let key = self.keys[self.v_registers[digits.1 as usize] as usize];
                if key {
                    self.program_counter += 2;
                }
            }

            (0xE, _, 0xA, 0x1) => {
                //SKIP KEY RELEASE - EXA1 : skip if key not pressed
                let key = self.keys[self.v_registers[digits.1 as usize] as usize];
                if !key {
                    self.program_counter += 2;
                }
            }

            (0xF, _, 0x0, 0x7) => {
                // VX = delay_timer - FX07 : stores in X the val of delay_timer
                self.v_registers[digits.1 as usize] = self.delay_timer;
            }

            (0xF, _, 0x0, 0xA) => {
                //WAIT - FX0A : wait until key press without async cause we dumb :3
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_registers[digits.1 as usize] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    //go back
                    self.program_counter -= 2;
                }
            }

            (0xF, _, 0x1, 0x5) => {
                //set delay_timer = VX - FX15
                self.delay_timer = self.v_registers[digits.1 as usize];
            }

            (0xF, _, 0x1, 0x8) => {
                //set sound_timer = VX - FX18
                self.sound_timer = self.v_registers[digits.2 as usize];
            }

            (0xF, _, 0x1, 0xE) => {
                // ram_index += VX - FX1E : WRAP add VX in ram_index
                self.ram_index = self
                    .ram_index
                    .wrapping_add(self.v_registers[digits.1 as usize] as u16);
            }

            (0xF, _, 0x2, 0x9) => {
                // ram_index = FONT - FX29 : load num sprite in ram_index
                self.ram_index = (self.v_registers[digits.1 as usize] as u16) * 5;
            }

            (0xF, _, 0x3, 0x3) => {
                //BCD - FX33 : this stuff is scary, usual number system shenanigans
                let val = self.v_registers[digits.1 as usize] as f32;
                let hundreds = (val / 100.0).floor() as u8;
                let tens = ((val / 10.0) % 10.0).floor() as u8;
                let ones = (val % 10.0).floor() as u8;

                self.ram[self.ram_index as usize] = hundreds;
                self.ram[(self.ram_index + 1) as usize] = tens;
                self.ram[(self.ram_index + 2) as usize] = ones;
            }

            (0xF, _, 0x5, 0x5) => {
                //STORE V0 - VX in ram - FX55
                let x = digits.1 as usize;
                let i = self.ram_index as usize;
                for idx in 0..=x {
                    self.ram[i + idx] = self.v_registers[idx];
                }
            }

            (0xF, _, 0x6, 0x5) => {
                //LOAD V0 - VX from ram - FX65
                let x = digits.1 as usize;
                let i = self.ram_index as usize;
                for idx in 0..=x {
                    self.v_registers[idx] = self.ram[i + idx];
                }
            }

            (_, _, _, _) => unimplemented!("Unimplemented error : opcode {}", op),
        }
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                //SOUND
            }
            self.sound_timer -= 1;
        }
    }

    //[stack behaviour]
    fn push(&mut self, val: u16) {
        self.stack[self.stack_pointer as usize] = val;
        self.stack_pointer += 1;
    }
    //panicking here is proper error handling OwO
    fn pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer as usize]
    }
}
