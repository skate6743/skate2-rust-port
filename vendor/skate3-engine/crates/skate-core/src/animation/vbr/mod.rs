//! Direct stock VBR SQT decoding from original Skate 3 TU3.
//! Source:82E88FA8/82E8A780(header),82E8A928(bits),82E89770(transform),
//! 82E89A60/82E88BA0(output), and82D20298(SQT channel descriptors).
//! Host allocation/cache ownership is independent. PC floating-point arithmetic
//! is not claimed to emulate the Xenon's numerical instructions bit for bit.
mod bits;
mod header;
mod transform;

pub struct VbrDecoder<'a> {
    bytes: &'a [u8],
    header: header::Header,
    cached_block: Option<(usize, Vec<[f32; 8]>)>,
}
impl<'a> VbrDecoder<'a> {
    pub fn new(
        bytes: &'a [u8],
        part_offset: usize,
        compression_header_relative: usize,
        compressed_data_relative: usize,
    ) -> Result<Self, String> {
        Ok(Self {
            bytes,
            header: header::Header::read(
                bytes,
                part_offset,
                compression_header_relative,
                compressed_data_relative,
            )?,
            cached_block: None,
        })
    }
    pub fn frame_count(&self) -> usize {
        self.header.frames
    }
    pub fn channel_count(&self) -> usize {
        self.header.bones
    }

    ///Return each authored bone's scaleXYZ, quaternionXYZW, translationXYZ bits.
    ///No quaternion normalization, retargeting, hierarchy changes or weight edits.
    pub fn decode_frame(&mut self, frame: usize) -> Result<Vec<[u32; 10]>, String> {
        if frame >= self.header.frames {
            return Err(format!("VBR frame {frame} out of range"));
        }
        let block_index = frame / 8;
        let block = &self.bytes[self.header.blocks[block_index].clone()];
        if self.cached_block.as_ref().map(|v| v.0) != Some(block_index) {
            self.cached_block =
                Some((block_index, bits::unpack(&block[3..], &self.header.widths)?));
        }
        let coefficients = &self.cached_block.as_ref().unwrap().1;
        let mut output = vec![[0; 10]; self.header.bones];
        let mut constant = 0;
        let mut dynamic = 0;
        let mut component_offset = 0;
        for (channel_index, channel) in self.header.channels.iter().enumerate() {
            for (bone, sqt) in output.iter_mut().enumerate() {
                let is_constant =
                    self.header.constant_flags[channel_index * self.header.bones + bone];
                for component in 0..channel.components {
                    let value = if is_constant {
                        let value = self.header.constants[constant];
                        constant += 1;
                        value
                    } else {
                        let value = transform::decode(
                            frame % 8,
                            block[channel_index],
                            self.header.dct_scale,
                            coefficients[dynamic],
                            channel.min,
                            channel.range,
                        );
                        dynamic += 1;
                        value
                    };
                    sqt[component_offset + component] = value.to_bits();
                }
            }
            component_offset += channel.components;
        }
        Ok(output)
    }
}
