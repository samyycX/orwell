use anyhow::Result;
use orwell::pb::orwell::{MessageType, ServerBroadcastMessage};


/// Context for message processing
pub struct MessageContext {
    pub is_history: bool,
}

/// Trait for message adapters
pub trait MessageAdapter: Send + Sync {
    /// Get the message type this adapter handles
    fn message_type(&self) -> MessageType;

    /// Process the message
    fn process(
        &self,
        message: &ServerBroadcastMessage,
        data: Vec<u8>,
        context: MessageContext,
    ) -> Result<()>;
}

/// Registry for message adapters
pub struct MessageAdapterRegistry {
    adapters: std::collections::HashMap<MessageType, Box<dyn MessageAdapter>>,
}

impl MessageAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn MessageAdapter>) {
        self.adapters.insert(adapter.message_type(), adapter);
    }

    pub fn get(&self, message_type: MessageType) -> Option<&dyn MessageAdapter> {
        self.adapters.get(&message_type).map(|a| a.as_ref())
    }

    pub fn process_message(
        &self,
        message: &ServerBroadcastMessage,
        data: Vec<u8>,
        context: MessageContext,
    ) -> Result<()> {
        let msg_type = MessageType::try_from(data[0] as i32)?;
        let actual_data = data[1..].to_vec();

        if let Some(adapter) = self.get(msg_type) {
            adapter.process(message, actual_data, context)
        } else {
            Ok(())
        }
    }
}
