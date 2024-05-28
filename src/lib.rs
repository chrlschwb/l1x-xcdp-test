// Import necessary crates and modules
use borsh::{BorshDeserialize, BorshSerialize};
use ethers::abi::{decode, ParamType};
use ethers::types::Log;
use l1x_sdk::{store::LookupMap, storage_read, storage_write};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

// Define constants for storage keys
const STORAGE_CONTRACT_KEY: &[u8; 7] = b"message";
const STORAGE_EVENTS_KEY: &[u8; 6] = b"events";

// Define data structures for messages
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct XCDPSendMessage {
    message: String,
}

// This structure is used for Solidity compatibility
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XCDPSendMessageSolidity {
    message: String,
}

// Conversion trait to allow easy transformations between Solidity and custom Rust structs
impl From<XCDPSendMessageSolidity> for XCDPSendMessage {
    fn from(event: XCDPSendMessageSolidity) -> Self {
        Self {
            message: event.message,
        }
    }
}

// Define the event structure manually without using EthEvent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XTalkMessageInitiated {
    message: Vec<u8>,
    destination_network: String,
    destination_smart_contract_address: [u8; 32],
}

// Payload structure for inter-chain messages
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Payload {
    data: Vec<u8>,
    destination_network: String,
    destination_contract_address: [u8; 32],
}

// Main contract structure storing all event data
#[derive(BorshSerialize, BorshDeserialize)]
pub struct XCDPCore {
    events: LookupMap<String, XCDPSendMessage>,
    total_events: u64,
}

// Default constructor for the contract
impl Default for XCDPCore {
    fn default() -> Self {
        Self {
            events: LookupMap::new(STORAGE_EVENTS_KEY.to_vec()),
            total_events: u64::default(),
        }
    }
}

impl XCDPCore {
    // Function to load existing contract data from storage
    fn load() -> Self {
        match storage_read(STORAGE_CONTRACT_KEY) {
            Some(bytes) => match Self::try_from_slice(&bytes) {
                Ok(contract) => contract,
                Err(_) => panic!("Unable to parse contract bytes"),
            },
            None => panic!("The contract isn't initialized"),
        }
    }

    // Function to save contract state to storage
    fn save(&mut self) {
        match self.try_to_vec() {
            Ok(encoded_contract) => {
                storage_write(STORAGE_CONTRACT_KEY, &encoded_contract);
                log::info!("Saved event data successfully");
            }
            Err(_) => panic!("Unable to save contract"),
        };
    }

    // Constructor to initialize a new contract
    pub fn new() {
        let mut contract = Self::default();
        contract.save();
    }

    // Handler to process incoming events and save the decoded data
    pub fn save_event_data(event_data: Vec<u8>, global_tx_id: String) {
        l1x_sdk::msg(&format!(
            "********************global tx id {} **************",
            global_tx_id
        ));

        let mut contract = Self::load();

        log::info!("Received event data!!!");
        assert!(!global_tx_id.is_empty(), "global_tx_id cannot be empty");
        assert!(!event_data.is_empty(), "event_data cannot be empty");
        assert!(
            !contract.events.contains_key(&global_tx_id),
            "event is saved already"
        );

        let event_data = match base64::decode(&event_data) {
            Ok(data) => data,
            Err(_) => panic!("Can't decode base64 event_data"),
        };

        let log: Log = serde_json::from_slice(&event_data).expect("Can't deserialize Log object");

        l1x_sdk::msg(&format!("{:#?}", log));
        let event_id = log.topics[0].to_string();
        let decoded_event_data = decode(
            &[ParamType::String],
            &log.data.0,
        )
        .unwrap();

        let event = XCDPSendMessageSolidity {
            message: decoded_event_data[0].clone().into_string().unwrap(),
        };

        contract.save_message_event(global_tx_id, event_id, event.into(), "destination_network_placeholder".to_string(), [0u8; 32]);

        contract.save()
    }

    // Function to combine parts of an event into a single storage key
    pub fn to_key(global_tx_id: String, event_type: String) -> String {
        global_tx_id + "-" + &event_type
    }

    // Function to save a message event
    pub fn save_message_event(&mut self, global_tx_id: String, event_id: String, event: XCDPSendMessage, destination_network: String, destination_smart_contract_address: [u8; 32]) {
        let key = Self::to_key(global_tx_id.clone(), event_id.clone());
        self.events.insert(key, event.clone());
        self.total_events += 1;
        log::info!(
            "Event saved with global_tx_id: {}, event_id: {}, message: {}, destination_network: {}, destination_smart_contract_address: {:?}",
            global_tx_id,
            event_id,
            event.message,
            destination_network,
            destination_smart_contract_address
        );
    }
}
