use uuid::Uuid;

pub struct Config {
    pub uuid: Uuid,
    pub host: String,
    pub outbound: String,
}
