use std::any::Any;
use json::{object, parse, stringify, JsonValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::Json;
use uuid::Uuid;

#[derive(Eq, Hash, PartialEq, Debug)]
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

impl DataTypes {
    pub fn parse(p0: String) -> DataTypes {
        // normalize: lowercase + remove underscores
        let normalized = p0.to_lowercase().replace('_', "");

        match normalized.as_str() {
            "errortype"        => DataTypes::ErrorType,
            "chatpartnerid"    => DataTypes::ChatPartnerId,
            "iotaid"           => DataTypes::IotaId,
            "userid"           => DataTypes::UserId,
            "userids"          => DataTypes::UserIds,
            "userstate"        => DataTypes::UserState,
            "userstates"       => DataTypes::UserStates,
            "userpings"        => DataTypes::UserPings,
            "callstate"        => DataTypes::CallState,
            "screenshare"      => DataTypes::ScreenShare,
            "privatekeyhash"   => DataTypes::PrivateKeyHash,
            "accepted"         => DataTypes::Accepted,
            "acceptedprofiles" => DataTypes::AcceptedProfiles,
            "deniedprofiles"   => DataTypes::DeniedProfiles,
            "messagecontent"   => DataTypes::MessageContent,
            "messagechunk"     => DataTypes::MessageChunk,
            "sendtime"         => DataTypes::SendTime,
            "gettime"          => DataTypes::GetTime,
            "getvariant"       => DataTypes::GetVariant,
            "sharedsecretown"  => DataTypes::SharedSecretOwn,
            "sharedsecretother"=> DataTypes::SharedSecretOther,
            "sharedsecretsign" => DataTypes::SharedSecretSign,
            "sharedsecret"     => DataTypes::SharedSecret,
            "callid"           => DataTypes::CallId,
            "callname"         => DataTypes::CallName,
            "callsecretsha"    => DataTypes::CallSecretSha,
            "callsecret"       => DataTypes::CallSecret,
            "sharedcallsecret" => DataTypes::SharedCallSecret,
            "startdate"        => DataTypes::StartDate,
            "enddate"          => DataTypes::EndDate,
            "receiverid"       => DataTypes::ReceiverId,
            "senderid"         => DataTypes::SenderId,
            "signature"        => DataTypes::Signature,
            "signed"           => DataTypes::Signed,
            "message"          => DataTypes::Message,
            "lastping"         => DataTypes::LastPing,
            "pingiota"         => DataTypes::PingIota,
            "pingclients"      => DataTypes::PingClients,
            "matches"          => DataTypes::Matches,
            "omikron"          => DataTypes::Omikron,
            "loadedmessages"   => DataTypes::LoadedMessages,
            "messageamount"    => DataTypes::MessageAmount,
            "position"         => DataTypes::Position,
            "name"             => DataTypes::Name,
            "path"             => DataTypes::Path,
            "codec"            => DataTypes::Codec,
            "function"         => DataTypes::Function,
            "payload"          => DataTypes::Payload,
            "result"           => DataTypes::Result,
            "interactables"    => DataTypes::Interactables,
            "wanttowatch"      => DataTypes::WantToWatch,
            "watcher"          => DataTypes::Watcher,
            "createdat"        => DataTypes::CreatedAt,
            "username"         => DataTypes::Username,
            "display"          => DataTypes::Display,
            "avatar"           => DataTypes::Avatar,
            "about"            => DataTypes::About,
            "status"           => DataTypes::Status,
            "publickey"        => DataTypes::PublicKey,
            "sublevel"         => DataTypes::SubLevel,
            "subend"           => DataTypes::SubEnd,
            "communityaddress" => DataTypes::CommunityAddress,
            "challenge"        => DataTypes::Challenge,
            "communitytitle"   => DataTypes::CommunityTitle,
            "communities"      => DataTypes::Communities,
            _ => DataTypes::ErrorType, // fallback if unknown
        }
    }
}

#[derive(PartialEq, Debug)]
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
impl CommunicationType {
    pub fn parse(p0: String) -> CommunicationType {
        let normalized = p0.to_lowercase().replace('_', "");

        match normalized.as_str() {
            "error" => CommunicationType::Error,
            "success" => CommunicationType::Success,
            "message" => CommunicationType::Message,
            "messagelive" => CommunicationType::MessageLive,
            "messageotheriota" => CommunicationType::MessageOtherIota,
            "messagechunk" => CommunicationType::MessageChunk,
            "messageget" => CommunicationType::MessageGet,
            "changeconfirm" => CommunicationType::ChangeConfirm,
            "confirmreceive" => CommunicationType::ConfirmReceive,
            "confirmread" => CommunicationType::ConfirmRead,
            "getchats" => CommunicationType::GetChats,
            "getstates" => CommunicationType::GetStates,
            "addcommunity" => CommunicationType::AddCommunity,
            "removecommunity" => CommunicationType::RemoveCommunity,
            "getcommunities" => CommunicationType::GetCommunities,
            "challenge" => CommunicationType::Challenge,
            "challengeresponse" => CommunicationType::ChallengeResponse,
            "register" => CommunicationType::Register,
            "registerresponse" => CommunicationType::RegisterResponse,
            "identification" => CommunicationType::Identification,
            "identificationresponse" => CommunicationType::IdentificationResponse,
            "ping" => CommunicationType::Ping,
            "pong" => CommunicationType::Pong,
            "addchat" => CommunicationType::AddChat,
            "sendchat" => CommunicationType::SendChat,
            "iotaconnected" => CommunicationType::IotaConnected,
            "iotaclosed" => CommunicationType::IotaClosed,
            "clientchanged" => CommunicationType::ClientChanged,
            "clientconnected" => CommunicationType::ClientConnected,
            "clientclosed" => CommunicationType::ClientClosed,
            "publickey" => CommunicationType::PublicKey,
            "privatekey" => CommunicationType::PrivateKey,
            "webrtcsdp" => CommunicationType::WebrtcSdp,
            "webrtcice" => CommunicationType::WebrtcIce,
            "startstream" => CommunicationType::StartStream,
            "endstream" => CommunicationType::EndStream,
            "watchstream" => CommunicationType::WatchStream,
            "getcall" => CommunicationType::GetCall,
            "newcall" => CommunicationType::NewCall,
            "callinvite" => CommunicationType::CallInvite,
            "endcall" => CommunicationType::EndCall,
            "function" => CommunicationType::Function,
            "update" => CommunicationType::Update,
            _ => CommunicationType::Error, // fallback
        }
    }
}
#[derive(Debug, Clone)]
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

    pub fn to_json(self) -> JsonValue {
        object! {
            message: self.message.clone(),
            log_level: self.log_level as i32
        }
    }
}
#[derive(Debug)]
pub struct CommunicationValue {
    pub id: Uuid,
    pub comm_type: CommunicationType,
    pub log_value: Option<LogValue>,
    pub sender: Option<Uuid>,
    pub receiver: Option<Uuid>,
    pub data: HashMap<DataTypes, String>,
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
    pub fn with_id(mut self, p0: Uuid) -> Self{
        self.id = p0;
        self
    }
    pub fn get_id(&self) -> Uuid{
        self.id.clone()
    }
    pub fn with_log(mut self, log: LogValue) -> Self {
        self.log_value = Some(log);
        self
    }
    pub fn get_log(&self) -> &Option<LogValue> {
        &self.log_value
    }
    pub fn with_sender(mut self, sender: Uuid) -> Self {
        self.sender = Some(sender);
        self
    }
    pub fn get_sender(&self) -> Option<Uuid> {
        self.sender.clone()
    }
    pub fn with_receiver(mut self, receiver: Uuid) -> Self {
        self.receiver = Some(receiver);
        self
    }
    pub fn get_receiver(&self) -> Option<Uuid> {
        self.receiver.clone()
    }
    pub fn add_data(mut self, key: DataTypes, value: String) -> Self {
        self.data.insert(key, value);
        self
    }
    pub fn get_data(&mut self, key: DataTypes) -> Option<&String> {
        self.data.get(&key)
    }

    pub(crate) fn is_type(&self, p0: CommunicationType) -> bool {
        self.comm_type == p0
    }
    pub fn to_json(&self) -> JsonValue {
        let mut jdata = object!{};
        for (k, v) in &self.data {
            jdata[&format!("{:?}", k)] = JsonValue::from(v.clone());
        }

        object! {
            id: self.id.to_string(),
            type: format!("{:?}", self.comm_type),
            sender: self.sender.map(|u| u.to_string()).unwrap_or_default(),
            receiver: self.receiver.map(|u| u.to_string()).unwrap_or_default(),
            log: self.log_value.as_ref().map(|l| l.clone().to_json()).unwrap_or(JsonValue::Null),
            data: jdata
        }
    }
 
    pub fn from_json(json_str: &str) -> Self {
        let parsed = parse(json_str).unwrap();

        let comm_type = CommunicationType::parse(parsed["type"].to_string());

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
        let uuid = Uuid::parse_str(parsed["id"].as_str().unwrap_or("")).unwrap_or(Uuid::new_v4());
        let mut data = HashMap::new();
        if parsed["data"].is_object() {
            for (k, v) in parsed["data"].entries() {
                if let Some(val) = v.as_str() {
                    data.insert(DataTypes::parse(k.to_string()), val.to_string());
                }
            }
        }
        Self {
            id: uuid,
            comm_type,
            log_value,
            sender,
            receiver,
            data,
        }
    }
    pub fn ack_message(message_id: Uuid, sender: Option<Uuid>) -> CommunicationValue {
        let mut cv = CommunicationValue::new(CommunicationType::Message)
            .with_id(message_id);

        if let Some(s) = sender {
            cv = cv.add_data(DataTypes::SenderId, s.to_string());
        }
        cv
    }
    pub fn forward_to_other_iota(original: &mut CommunicationValue) -> CommunicationValue {
        let receiver = Uuid::from_str(original.get_data(DataTypes::ReceiverId).unwrap()).ok()
            .or(Option::from(Uuid::nil()));

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let mut cv = CommunicationValue::new(CommunicationType::MessageOtherIota)
            .with_id(original.get_id())
            .with_receiver(receiver.unwrap())
            .add_data(DataTypes::SendTime, now_ms.to_string())
            .add_data(DataTypes::MessageContent, original.get_data(DataTypes::MessageContent).unwrap().to_string());

        // include sender_id if the original had one
        if let Some(sender) = original.get_sender() {
            cv = cv.add_data(DataTypes::SenderId, sender.to_string());
        }
        cv
    }
}