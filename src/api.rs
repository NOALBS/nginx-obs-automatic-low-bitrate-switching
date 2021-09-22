use tracing::info;

use crate::user_manager::UserManager;

pub async fn run(user_manager: UserManager, port: u16) {
    let api = filters::routes(user_manager);

    info!("Running API on 127.0.0.1:{}", port);
    warp::serve(api).run(([127, 0, 0, 1], port)).await;
}

mod filters {
    use reqwest::Method;
    use warp::Filter;

    use super::handlers;
    use crate::user_manager::UserManager;

    pub fn routes(
        user_manager: UserManager,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(&[Method::GET]);

        noalbs_users(user_manager.clone())
            .or(noalbs_user(user_manager.clone()))
            .or(noalbs_ws(user_manager))
            .with(cors)
    }

    pub fn noalbs_users(
        user_manager: UserManager,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("users")
            .and(warp::get())
            .and(with_db(user_manager))
            .and_then(handlers::get_users)
    }

    pub fn noalbs_user(
        user_manager: UserManager,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("users" / String)
            .and(warp::get())
            .and(with_db(user_manager))
            .and_then(handlers::get_user)
    }

    pub fn noalbs_ws(
        user_manager: UserManager,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("ws")
            .and(warp::ws())
            .and(with_db(user_manager))
            .and_then(handlers::ws_client)
    }

    fn with_db(
        user_manager: UserManager,
    ) -> impl Filter<Extract = (UserManager,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || user_manager.clone())
    }
}

mod handlers {
    use std::convert::Infallible;

    //use warp::http::StatusCode;

    use crate::{config::User, user_manager::UserManager, ws};

    pub async fn get_users(user_manager: UserManager) -> Result<impl warp::Reply, Infallible> {
        let db = user_manager.get();
        let users = db.read().await;

        let mut all_users = Vec::new();

        for k in (*users).keys() {
            all_users.push(k);
        }

        Ok(warp::reply::json(&all_users))
    }

    pub async fn get_user(
        name: String,
        user_manager: UserManager,
    ) -> Result<impl warp::Reply, Infallible> {
        let db = user_manager.get();
        let users = db.read().await;

        let found_user = users.get(&name);

        let found_user = match found_user {
            Some(user) => user,
            None => todo!(),
        };

        let state = found_user.state.read().await;

        Ok(warp::reply::json(&state.config))
    }

    pub async fn ws_client(
        ws: warp::ws::Ws,
        user_manager: UserManager,
    ) -> Result<impl warp::Reply, Infallible> {
        Ok(ws.on_upgrade(|websocket| ws::user_connected(websocket, user_manager)))
    }
}
