use rustot::ota::pal::{OtaPal, OtaPalError, PalImageState};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Cursor, Write};

pub struct FileHandler {
    filebuf: Option<Cursor<Vec<u8>>>,
}

impl FileHandler {
    pub fn new() -> Self {
        FileHandler { filebuf: None }
    }
}

impl OtaPal for FileHandler {
    type Error = ();

    fn abort(
        &mut self,
        _file: &rustot::ota::encoding::FileContext,
    ) -> Result<(), OtaPalError<Self::Error>> {
        Ok(())
    }

    fn create_file_for_rx(
        &mut self,
        file: &rustot::ota::encoding::FileContext,
    ) -> Result<(), OtaPalError<Self::Error>> {
        self.filebuf = Some(Cursor::new(Vec::with_capacity(file.filesize)));
        Ok(())
    }

    fn get_platform_image_state(&self) -> Result<PalImageState, OtaPalError<Self::Error>> {
        Ok(PalImageState::Valid)
    }

    fn set_platform_image_state(
        &mut self,
        _image_state: rustot::ota::pal::ImageState,
    ) -> Result<(), OtaPalError<Self::Error>> {
        Ok(())
    }

    fn reset_device(&mut self) -> Result<(), OtaPalError<Self::Error>> {
        Ok(())
    }

    fn close_file(
        &mut self,
        file: &rustot::ota::encoding::FileContext,
    ) -> Result<(), OtaPalError<Self::Error>> {
        if let Some(ref mut buf) = &mut self.filebuf {
            let mut hasher = Sha256::new();
            hasher.update(buf.get_ref());
            println!("Sha256 is {:?}!", hasher.finalize());

            let mut file =
                File::create(file.filepath.as_str()).map_err(|_| OtaPalError::FileWriteFailed)?;
            file.write_all(buf.get_ref())
                .map_err(|_| OtaPalError::FileWriteFailed)?;

            Ok(())
        } else {
            Err(OtaPalError::BadFileHandle)
        }
    }

    fn write_block(
        &mut self,
        _file: &rustot::ota::encoding::FileContext,
        block_offset: usize,
        block_payload: &[u8],
    ) -> Result<usize, OtaPalError<Self::Error>> {
        if let Some(ref mut buf) = &mut self.filebuf {
            buf.set_position(block_offset as u64);
            buf.write(block_payload)
                .map_err(|_e| OtaPalError::FileWriteFailed)?;
            Ok(block_payload.len())
        } else {
            Err(OtaPalError::BadFileHandle)
        }
    }

    fn get_active_firmware_version(
        &self,
    ) -> Result<rustot::ota::pal::Version, OtaPalError<Self::Error>> {
        Ok(rustot::ota::pal::Version::new(0, 1, 0))
    }
}
