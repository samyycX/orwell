use std::{collections::HashMap, sync::Arc};

use diesel::prelude::*;
use lazy_static::lazy_static;
use orwell::{
    pb::orwell::{ClientInfo as PbClientInfo, ClientStatus},
    schema::clients_::{self, dsl::*},
};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::get_db_connection;

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = clients_)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(primary_key(id_))]
pub struct Client {
    pub id_: String,
    pub name_: String,
    pub kyber_pk_: Vec<u8>,
    pub dilithium_pk_: Vec<u8>,
    pub online_time_: i64,
    pub color_: i32,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.id_ == other.id_
    }
}

impl Default for Client {
    fn default() -> Self {
        Client {
            id_: "".to_string(),
            name_: "".to_string(),
            kyber_pk_: vec![],
            dilithium_pk_: vec![],
            online_time_: 0,
            color_: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub client: Client,
    pub status: ClientStatus,
}

impl ClientInfo {
    pub fn to_pb_client_info(&self) -> PbClientInfo {
        PbClientInfo {
            id: self.client.id_.clone(),
            name: self.client.name_.clone(),
            color: self.client.color_ as u32,
            kyber_pk: self.client.kyber_pk_.clone(),
            status: self.status as i32,
        }
    }
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            client: Client::default(),
            status: ClientStatus::Online,
        }
    }
}

lazy_static! {
    pub static ref CLIENT_MANAGER: Arc<RwLock<ClientManager>> =
        Arc::new(RwLock::new(ClientManager::new()));
}
pub struct ClientManager {
    pub clients: HashMap<u32, ClientInfo>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn is_name_taken(name: &str) -> bool {
        let mut conn: SqliteConnection = get_db_connection();
        clients_
            .filter(name_.eq(name))
            .first::<Client>(&mut conn)
            .optional()
            .unwrap()
            .is_some()
    }

    pub fn register_client(name: &str, kyber_pk: &[u8], dilithium_pk: &[u8], color: i32) -> Client {
        let id = Uuid::now_v7().to_string();
        let client = Client {
            id_: id,
            name_: name.to_string(),
            kyber_pk_: kyber_pk.to_vec(),
            dilithium_pk_: dilithium_pk.to_vec(),
            color_: color,
            online_time_: 0,
        };
        let mut conn = get_db_connection();
        diesel::insert_into(clients_)
            .values(client.clone())
            .execute(&mut conn)
            .unwrap();
        client
    }

    pub fn find_client(dilithium_pk: &[u8]) -> Option<Client> {
        let mut conn: SqliteConnection = get_db_connection();
        clients_
            .filter(dilithium_pk_.eq(dilithium_pk))
            .first::<Client>(&mut conn)
            .optional()
            .unwrap()
    }

    pub async fn login_client(conn_id: u32, client: Client) -> ClientInfo {
        let mut client_manager = CLIENT_MANAGER.write().await;
        let info = ClientInfo {
            client: client,
            status: ClientStatus::Online,
        };
        client_manager.clients.insert(conn_id, info.clone());
        info
    }

    pub async fn get_client_by_id(id: &str) -> Option<Client> {
        let mut conn: SqliteConnection = get_db_connection();
        clients_
            .filter(id_.eq(id))
            .first::<Client>(&mut conn)
            .optional()
            .unwrap()
    }

    pub async fn get_online_client_by_connection(conn_id: u32) -> Option<ClientInfo> {
        let client_manager = CLIENT_MANAGER.read().await;
        let client = client_manager.clients.get(&conn_id);
        if client.is_none() {
            return None;
        };
        Some(client.unwrap().clone())
    }

    pub async fn get_client_connection_by_id(id: &str) -> Option<u32> {
        let client_manager = CLIENT_MANAGER.read().await;
        for (conn_id, client_info) in client_manager.clients.iter() {
            if client_info.client.id_ == id {
                return Some(*conn_id);
            }
        }
        None
    }

    pub async fn get_client_by_connection(conn_id: u32) -> Option<ClientInfo> {
        let client_manager = CLIENT_MANAGER.read().await;
        let client_info = client_manager.clients.get(&conn_id);
        if client_info.is_none() {
            return None;
        }
        Some(client_info.unwrap().clone())
    }

    pub async fn remove_connection(conn_id: u32) {
        let mut client_manager = CLIENT_MANAGER.write().await;
        client_manager.clients.remove(&conn_id);
    }

    pub async fn get_all_online_clients() -> Vec<ClientInfo> {
        let client_manager: tokio::sync::RwLockReadGuard<'_, ClientManager> =
            CLIENT_MANAGER.read().await;
        client_manager.clients.values().cloned().collect()
    }

    pub async fn get_all_clients() -> Vec<ClientInfo> {
        let client_manager = CLIENT_MANAGER.read().await;
        let online_clients = client_manager.clients.values().cloned().collect::<Vec<_>>();

        // get offline clients
        let mut conn: SqliteConnection = get_db_connection();
        let all_clients = clients_.load::<Client>(&mut conn).unwrap();
        let offline_clients = all_clients
            .iter()
            .filter(|client| !online_clients.iter().any(|c| c.client.id_ == client.id_))
            .map(|client| ClientInfo {
                client: client.clone(),
                status: ClientStatus::Offline,
            })
            .collect::<Vec<_>>();
        info!("online_clients: {:?}", offline_clients.len());
        online_clients.into_iter().chain(offline_clients).collect()
    }

    pub async fn get_all_connections() -> Vec<u32> {
        let client_manager = CLIENT_MANAGER.read().await;
        client_manager.clients.keys().cloned().collect()
    }

    pub async fn update_color(id: &str, color: i32) {
        let mut conn: SqliteConnection = get_db_connection();
        diesel::update(clients_)
            .filter(id_.eq(id))
            .set(color_.eq(color))
            .execute(&mut conn)
            .unwrap();

        if let Some(client) = Self::get_client_connection_by_id(id).await {
            let mut client_manager = CLIENT_MANAGER.write().await;
            client_manager
                .clients
                .get_mut(&client)
                .unwrap()
                .client
                .color_ = color;
        }
    }

    pub async fn get_status(conn_id: u32) -> ClientStatus {
        let client_manager = CLIENT_MANAGER.read().await;
        client_manager.clients.get(&conn_id).unwrap().status
    }

    pub async fn update_status(conn_id: u32, status: ClientStatus) {
        let mut client_manager = CLIENT_MANAGER.write().await;
        client_manager.clients.get_mut(&conn_id).unwrap().status = status;
    }
}
