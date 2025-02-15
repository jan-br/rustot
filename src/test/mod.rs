use std::{cell::RefCell, collections::VecDeque};

use mqttrust::{encoding::v4::encode_slice, Mqtt, MqttError, Packet};

///
/// Mock Mqtt client used for unit tests. Implements `mqttrust::Mqtt` trait.
///
pub struct MockMqtt {
    pub tx: RefCell<VecDeque<Vec<u8>>>,
    publish_fail: bool,
}

impl MockMqtt {
    pub fn new() -> Self {
        Self {
            tx: RefCell::new(VecDeque::new()),
            publish_fail: false,
        }
    }

    pub fn publish_fail(&mut self) {
        self.publish_fail = true;
    }
}

impl Mqtt for MockMqtt {
    fn send(&self, packet: Packet<'_>) -> Result<(), MqttError> {
        let v = &mut [0u8; 1024];

        let len = encode_slice(&packet, v).map_err(|_| MqttError::Full)?;
        let packet = v[..len].iter().cloned().collect();
        self.tx.borrow_mut().push_back(packet);

        Ok(())
    }

    fn client_id(&self) -> &str {
        "test_client"
    }
}
