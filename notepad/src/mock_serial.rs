//! Mock serial port for unit testing
use std::io::Write;

#[derive(Clone)]
pub struct MockSerial {
    buffer: Vec<u8>,
}

impl std::default::Default for MockSerial {
    fn default() -> Self {
        MockSerial { buffer: vec![] }
    }
}

impl std::io::Write for MockSerial {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(self.buffer.write(buf)?)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for MockSerial {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_read = buf.write(&self.buffer[..])?;
        let _ = self.buffer.drain(..bytes_read).collect::<Vec<u8>>();

        Ok(bytes_read)
    }
}

impl serialport::SerialPort for MockSerial {
    fn name(&self) -> Option<String> {
        None
    }
    fn baud_rate(&self) -> serialport::Result<u32> {
        Ok(42)
    }
    fn data_bits(&self) -> serialport::Result<serialport::DataBits> {
        Ok(serialport::DataBits::Eight)
    }
    fn flow_control(&self) -> serialport::Result<serialport::FlowControl> {
        Ok(serialport::FlowControl::None)
    }
    fn parity(&self) -> serialport::Result<serialport::Parity> {
        Ok(serialport::Parity::None)
    }
    fn stop_bits(&self) -> serialport::Result<serialport::StopBits> {
        Ok(serialport::StopBits::One)
    }
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::new(1, 0)
    }
    fn set_baud_rate(&mut self, _baud_rate: u32) -> serialport::Result<()> {
        Ok(())
    }
    fn set_data_bits(&mut self, _data_bits: serialport::DataBits) -> serialport::Result<()> {
        Ok(())
    }
    fn set_flow_control(
        &mut self,
        _flow_control: serialport::FlowControl,
    ) -> serialport::Result<()> {
        Ok(())
    }
    fn set_parity(&mut self, _parity: serialport::Parity) -> serialport::Result<()> {
        Ok(())
    }
    fn set_stop_bits(&mut self, _stop_bits: serialport::StopBits) -> serialport::Result<()> {
        Ok(())
    }
    fn set_timeout(&mut self, _timeout: std::time::Duration) -> serialport::Result<()> {
        Ok(())
    }
    fn write_request_to_send(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }
    fn write_data_terminal_ready(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }
    fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }
    fn bytes_to_read(&self) -> serialport::Result<u32> {
        Ok(42)
    }
    fn bytes_to_write(&self) -> serialport::Result<u32> {
        Ok(42)
    }
    fn clear(&self, _buffer_to_clear: serialport::ClearBuffer) -> serialport::Result<()> {
        Ok(())
    }
    fn try_clone(&self) -> serialport::Result<Box<dyn serialport::SerialPort>> {
        Ok(Box::new(self.clone()))
    }
    fn set_break(&self) -> serialport::Result<()> {
        Ok(())
    }
    fn clear_break(&self) -> serialport::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test_mock_serial {
    use super::*;
    use serialport::SerialPort;

    #[test]
    fn call_functions() {
        let mut ms = MockSerial::default();

        ms.name();
        ms.baud_rate().unwrap();
        ms.data_bits().unwrap();
        ms.flow_control().unwrap();
        ms.parity().unwrap();
        ms.stop_bits().unwrap();
        ms.timeout();
        ms.set_baud_rate(42).unwrap();
        ms.set_data_bits(serialport::DataBits::Seven).unwrap();
        ms.set_flow_control(serialport::FlowControl::None).unwrap();
        ms.set_parity(serialport::Parity::None).unwrap();
        ms.set_stop_bits(serialport::StopBits::One).unwrap();
        ms.set_timeout(std::time::Duration::new(1, 0)).unwrap();
        ms.write_request_to_send(false).unwrap();
        ms.write_data_terminal_ready(false).unwrap();
        ms.read_clear_to_send().unwrap();
        ms.read_data_set_ready().unwrap();
        ms.read_ring_indicator().unwrap();
        ms.read_carrier_detect().unwrap();
        ms.bytes_to_read().unwrap();
        ms.bytes_to_write().unwrap();
        ms.clear(serialport::ClearBuffer::All).unwrap();
        ms.try_clone().unwrap();
        ms.set_break().unwrap();
        ms.clear_break().unwrap();
    }

    #[test]
    fn write() {
        use std::io::Write;

        let mut ms = MockSerial::default();
        write!(ms, "hello").unwrap();
        ms.flush().unwrap();
    }

    #[test]
    fn read() {
        use std::io::Read;

        let mut ms = MockSerial::default();
        let mut buf: [u8; 8] = [0; 8];
        ms.read(&mut buf).unwrap();
    }

    #[test]
    fn read_write() {
        use std::io::{Read, Write};

        let mut ms = MockSerial::default();
        let mut buf: [u8; 4] = [0; 4];

        write!(ms, "test").unwrap();
        write!(ms, "t3st").unwrap();

        ms.read(&mut buf).unwrap();
        assert_eq!(buf, "test".as_bytes());
        ms.read(&mut buf).unwrap();
        assert_eq!(buf, "t3st".as_bytes());
    }
}
