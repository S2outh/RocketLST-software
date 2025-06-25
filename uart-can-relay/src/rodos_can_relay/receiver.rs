use embassy_stm32::can::{BufferedCanReceiver, Frame};
use embedded_can::Id;
use heapless::{FnvIndexMap, Vec};

use super::common::*;

struct RodosCanFramePart {
    topic: u16,
    device: u8,
    data: [u8;5],
    seq_num: u8,
    seq_len: u8,
}

/// Module to send messages on a rodos can
pub struct RodosCanReceiver {
    receiver: BufferedCanReceiver,
    partial_frames: FnvIndexMap<u32, Vec<RodosCanFramePart, {u8::MAX as usize}>, 16>
}

impl RodosCanReceiver {
    /// create a new instance from BufferedCanReceiver
    pub(super) fn new(receiver: BufferedCanReceiver) -> Self {
        RodosCanReceiver { receiver, partial_frames: FnvIndexMap::new() }
    }
    /// take a can hal frame and decode it to RODOS message parts
    fn decode(frame: &Frame) -> Result<(u32, RodosCanFramePart), RodosCanDecodeError> {
        let Id::Extended(id) = frame.id() else {
            return Err(RodosCanDecodeError::WrongIDType);
        };
        let id = id.as_raw();
        let topic = (id >> 8) as u16;
        let device = id as u8;
        
        if frame.data().len() <= 3 {
            // No data in can msg
            return Err(RodosCanDecodeError::NoData);
        }
        let seq_num = frame.data()[0];
        let seq_len = frame.data()[2];
        let data = frame.data()[2..].try_into().unwrap();
        
        Ok((id, RodosCanFramePart { topic, device, data, seq_num, seq_len }))
    }
    /// receive the next rodos frame async
    pub async fn receive(&mut self) -> Result<RodosCanFrame, RodosCanError> {
        loop {
            match self.receiver.receive().await {
                Ok(envelope) => {
                    let (id, frame_part) = Self::decode(&envelope.frame).map_err(|e| RodosCanError::CouldNotDecode(e))?;
                    if !self.partial_frames.contains_key(&id) {
                        let mut vec = Vec::new();
                        vec.push(frame_part);
                        self.partial_frames.insert(id, vec);
                    }
                }
                Err(e) => return Err(RodosCanError::BusError(e))
            }
        }
    }
}
