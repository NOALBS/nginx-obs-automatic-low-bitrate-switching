use crate::{broadcasting_software, chat, db, switcher, Error, VERSION};
use actix_web::{dev, get, web, App, HttpResponse, HttpServer, Responder};
use log::info;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::Noalbs;

pub struct Api {
    pub http_server: dev::Server,
}

impl Api {
    pub async fn run(all_clients: Arc<RwLock<HashMap<i64, Noalbs>>>) -> Result<Api, Error> {
        info!("Starting API");
        let ac = web::Data::new(all_clients);
        //let db_con = Db::connect().await?;

        let srv = HttpServer::new(move || {
            App::new()
                //.app_data(db_con.clone())
                .app_data(ac.clone())
                .service(user)
                .service(users)
                .service(version)
        })
        .bind("127.0.0.1:8080")?
        .run();

        Ok(Api { http_server: srv })
    }
}

#[derive(Serialize)]
struct NoalbsInfo<'a> {
    pub id: &'a i64,
    pub username: &'a String,
    pub broadcasting_software_state: &'a broadcasting_software::State,
    pub switcher_state: &'a switcher::SwitcherState,
    pub chat_state: &'a chat::State,
    pub connections: &'a Vec<db::Connection>,
}

#[derive(Serialize)]
struct User<'a> {
    pub id: &'a i64,
    pub username: &'a String,
}

#[get("/users")]
async fn users(data: web::Data<Arc<RwLock<HashMap<i64, Noalbs>>>>) -> impl Responder {
    let mut users = Vec::new();

    let data = data.read().await;

    for (_, noalbs) in data.iter() {
        let info = User {
            id: &noalbs.user.id,
            username: &noalbs.user.username,
        };

        users.push(info);
    }

    HttpResponse::Ok().json(users)
}

#[get("/users/{id}")]
async fn user(
    path: web::Path<i64>,
    data: web::Data<Arc<RwLock<HashMap<i64, Noalbs>>>>,
) -> impl Responder {
    let data = data.read().await;

    // TODO: Change unwrap into error
    let user = data.get(&path).unwrap();

    let bs_state = user.broadcasting_software.read().await.state();

    let info = NoalbsInfo {
        id: &user.user.id,
        username: &user.user.username,
        broadcasting_software_state: &*bs_state.lock().await,
        switcher_state: &*user.switcher_state.lock().await,
        chat_state: &*user.chat_state.lock().await,
        connections: &user.connections,
    };

    HttpResponse::Ok().json(info)
}

#[get("/version")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok().body(format!("Running NOALBS v{}", VERSION))
}

// pub async fn create_user() -> impl Responder {
//     info!("Creating user {}");
//
//     let id = db_con.create_user("715209").await?;
//
//     let connection = db::Connection {
//         channel: "715209".to_string(),
//         platform: db::Platform::Twitch,
//     };
//
//     db_con.add_connection(id, connection).await?;
//     users = db_con.get_users().await?;
// }
