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
      Ok(input.to_vec())
    } else {
      let mut output = vec![0u8; input.len().max(1) * 4];
      let mut consumed = 0;
      let call_start_in = self.decompress.total_in();
      let call_start_out = self.decompress.total_out();

      loop {
        let status = self
          .decompress
          .decompress(
            &input[consumed..],
            &mut output[(self.decompress.total_out() - call_start_out) as usize..],
            FlushDecompress::Sync
          )
          .map_err(|e| format!("MCCP2 decompression error: {e}"))?;

        consumed = (self.decompress.total_in() - call_start_in) as usize;

        match status {
          flate2::Status::Ok => {
            if consumed >= input.len() {
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

      let total_out = (self.decompress.total_out() - call_start_out) as usize;
      output.truncate(total_out);
      Ok(output)
    }
  }
}
