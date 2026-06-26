use flate2::{Decompress, FlushDecompress};

pub struct MccpDecompressor {
  decompress: Decompress,
  active: bool
}

impl MccpDecompressor {
  pub fn new() -> Self { Self { decompress: Decompress::new(true), active: false } }

  pub fn activate(&mut self) {
    self.decompress.reset(true);
    self.active = true;
  }

  pub fn is_active(&self) -> bool { self.active }

  pub fn decompress_data(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
    if !self.active {
      return Ok(input.to_vec());
    }

    let mut output = vec![0u8; input.len() * 4];
    let mut total_out = 0;

    loop {
      let before_in = self.decompress.total_in();
      let before_out = self.decompress.total_out();

      let status = self
        .decompress
        .decompress(
          &input[(before_in as usize).min(input.len())..],
          &mut output[total_out..],
          FlushDecompress::Sync
        )
        .map_err(|e| format!("MCCP2 decompression error: {e}"))?;

      total_out = (self.decompress.total_out() - before_out + total_out as u64) as usize;

      match status {
        flate2::Status::Ok => {
          if self.decompress.total_in() as usize >= input.len() {
            break;
          }
          output.resize(output.len() + input.len() * 2, 0);
        }
        flate2::Status::StreamEnd => {
          self.active = false;
          break;
        }
        flate2::Status::BufError => break
      }
    }

    output.truncate(total_out);
    Ok(output)
  }
}
