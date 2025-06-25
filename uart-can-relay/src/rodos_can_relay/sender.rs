use embassy_stm32::can::BufferedCanSender;

/// Module to receive messages from a rodos can
pub struct RodosCanSender {
    sender: BufferedCanSender,
}


impl RodosCanSender {
    /// create a new instance from BufferedCanSender
    pub(super) fn new(sender: BufferedCanSender) -> Self {
        RodosCanSender { sender }
    }
}
