use defmt::Format;
use embassy_stm32::can::BufferedCanSender;
use embedded_can::{ExtendedId, Frame};
use heapless::Vec;

use super::RODOS_CAN_ID;

/// Module to receive messages from RODOS over can
pub struct RodosCanSender {
    sender: BufferedCanSender,
    device_id: u8,
}

/// Error enum for the all RODOS can sending operations
#[derive(Format)]
pub enum RodosCanSendError {
    /// One Rodos can frame can have a max length of u8::MAX * 5 bytes
    ToMuchData,
}

impl RodosCanSender {
    /// create a new instance from BufferedCanSender
    pub(super) fn new(sender: BufferedCanSender, device_id: u8) -> Self {
        RodosCanSender { sender, device_id }
    }
    /// takes a topic and device and returns a RODOS id
    fn encode_id(&self, topic: u16) -> u32 {
        return (RODOS_CAN_ID as u32) << (16 + 8) | (topic as u32) << 8 | self.device_id as u32;
    }
    /// send a rodos frame async
    pub async fn send(&mut self, topic: u16, data: &[u8]) -> Result<(), RodosCanSendError> {
        let id = ExtendedId::new(self.encode_id(topic)).unwrap();

        // split data into chunks of 5 bytes
        let mut frame_data_chunks = data.chunks(5);

        // if frame too long return an error
        if frame_data_chunks.len() > u8::MAX as usize {
            return Err(RodosCanSendError::ToMuchData);
        }
        let mut frame_index: u8 = 0;
        while let Some(frame_data) = frame_data_chunks.next() {
            // create the frame header
            let frame_header = [frame_index, 0x00, frame_data_chunks.len() as u8];

            // create frame
            let mut frame = Vec::<_, 8>::from_slice(&frame_header).unwrap();
            frame.extend_from_slice(frame_data).unwrap();

            // send on can
            let can_frame = Frame::new(id, &frame).unwrap();
            self.sender.write(can_frame).await;
            frame_index += 1;
        }

        Ok(())
    }
}
