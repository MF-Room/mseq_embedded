use stm32f4xx_hal::block;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Driver: Error while writing to `{0}`.")]
    Write(&'static str),
}

pub fn write<U>(tx: &mut U, bytes: &[u8]) -> Result<(), DriverError>
where
    U: embedded_hal_nb::serial::Write<u8>,
{
    for &b in bytes {
        block!(tx.write(b)).map_err(|_| DriverError::Write("Serial connection"))?;
    }
    Ok(())
}
