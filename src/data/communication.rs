use json::{object, JsonValue};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataTypes {
    ErrorType,               
    ChatPartnerId,           
    IotaId,
    UserId,
    UserIds,
    UserState,
    UserStates,
    UserPings,
    CallState,
    ScreenShare,
    PrivateKeyHash,
    Accepted,
    AcceptedProfiles,
    DeniedProfiles,
    MessageContent,
    MessageChunk,
    SendTime,
    GetTime,
    GetVariant,
    SharedSecretOwn,
    SharedSecretOther,
    SharedSecretSign,
    SharedSecret,
    CallId,
    CallName,
    CallSecretSha,
    CallSecret,
    SharedCallSecret,
    StartDate,
    EndDate,
    ReceiverId,
    SenderId,
    Signature,
    Signed,
    Message,
    LastPing,
    PingIota,
    PingClients,
    Matches,
    Omikron,
    LoadedMessages,
    MessageAmount,
    Position,
    Name,
    Path,
    Codec,
    Function,
    Payload,
    Result,
    Interactables,
    WantToWatch,
    Watcher,
    CreatedAt,
    Username,
    Display,
    Avatar,
    About,
    Status,
    PublicKey,
    SubLevel,
    SubEnd,
    CommunityAddress,
    Challenge,
    CommunityTitle,
    Communities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommunicationType {
Error,
    Success,
    Message,
    MessageLive,
    MessageOtherIota,
    MessageChunk,
    MessageGet,
    ChangeConfirm,
    ConfirmReceive,
    ConfirmRead,
    GetChats,
    GetStates,
    AddCommunity,
    RemoveCommunity,
    GetCommunities,
    Challenge,
    ChallengeResponse,
    Register,
    RegisterResponse,
    Identification,
    IdentificationResponse,
    Ping,
    Pong,
    AddChat,
    SendChat,
    IotaConnected,
    IotaClosed,
    ClientChanged,
    ClientConnected,
    ClientClosed,
    PublicKey,
    PrivateKey,
    WebrtcSdp,
    WebrtcIce,
    StartStream,
    EndStream,
    WatchStream,
    GetCall,
    NewCall,
    CallInvite,
    EndCall,
    Function,
    Update,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Important = 2,
    Normal = 1,
    Debug = 0,
    None = -1,
    DebugOnly = -2,
}

#[derive(Debug, Clone)]
pub struct LogValue {
    pub message: String,
    pub log_level: LogLevel,
}

impl LogValue {
    pub fn new(message: impl Into<String>, log_level: LogLevel) -> Self {
        Self {
            message: message.into(),
            log_level,
        }
    }

    pub fn to_json(&self) -> JsonValue {
        object! {
            message: self.message.clone(),
            log_level: self.log_level.clone() as i32
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommunicationValue {
    pub id: Uuid,
    pub comm_type: CommunicationType,
    pub log_value: Option<LogValue>,
    pub sender: Option<Uuid>,
    pub receiver: Option<Uuid>,
    pub data: HashMap<DataTypes, JsonValue>,
}

impl CommunicationValue {
    pub fn new(comm_type: CommunicationType) -> Self {
        Self {
            id: Uuid::new_v4(),
            comm_type,
            log_value: None,
            sender: None,
            receiver: None,
            data: HashMap::new(),
        }
    }

    pub fn with_log(mut self, log: LogValue) -> Self {
        self.log_value = Some(log);
        self
    }

    pub fn with_sender(mut self, sender: Uuid) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_receiver(mut self, receiver: Uuid) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn add_data(mut self, key: DataTypes, value: JsonValue) -> Self {
        self.data.insert(key, value);
        self
    }

    pub fn to_json(&self) -> JsonValue {
        let mut jdata = object!{};
        for (k, v) in &self.data {
            jdata[&format!("{:?}", k)] = v.clone();
        }

        object! {
            id: self.id.to_string(),
            type: format!("{:?}", self.comm_type),
            sender: self.sender.map(|u| u.to_string()).unwrap_or_default(),
            receiver: self.receiver.map(|u| u.to_string()).unwrap_or_default(),
            log: self.log_value.as_ref().map(|l| l.to_json()).unwrap_or(JsonValue::Null),
            data: jdata
        }
    }
 
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let parsed = json::parse(json_str).map_err(|e| e.to_string())?;

        let message_id = parsed["id"].as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(|| Uuid::new_v4());

        let comm_type = match parsed["type"].as_str() {
            Some("Message") => CommunicationType::Message,
            Some("Success") => CommunicationType::Success,
            _ => CommunicationType::Error,
        };

        let sender = parsed["sender"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        let receiver = parsed["receiver"].as_str().and_then(|s| Uuid::parse_str(s).ok());

        let log_value = if parsed["log"].is_object() {
            Some(LogValue::new(
                parsed["log"]["message"].as_str().unwrap_or("").to_string(),
                LogLevel::Normal,
            ))
        } else {
            None
        };

        Ok(Self {
            id: message_id,
            comm_type,
            log_value,
            sender,
            receiver,
            data: HashMap::new(),
        })
    }
}