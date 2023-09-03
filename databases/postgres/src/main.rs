mod config {
    use serde::Deserialize;
    #[derive(Debug, Default, Deserialize)]
    pub struct ExampleConfig {
        pub server_addr: String,
        pub pg: deadpool_postgres::Config,
    }
}

mod models {
    use serde::{Deserialize, Serialize, };
    use tokio_pg_mapper_derive::PostgresMapper;

    #[derive(Deserialize, PostgresMapper, Serialize, Debug)]
    #[pg_mapper(table = "users")] // singular 'user' is a keyword..
    pub struct User {
        pub username:String,
        pub pwd:String,
        pub email: String,
        pub phone:String,
        // pub first_name: String,
        // pub last_name: String,
        // pub username: String,
    }
}

mod errors {
    use actix_web::{HttpResponse, ResponseError};
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum MyError {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }
    impl std::error::Error for MyError {}

    impl ResponseError for MyError {
        fn error_response(&self) -> HttpResponse {
            match *self {
                MyError::NotFound => HttpResponse::NotFound().finish(),
                MyError::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}

use serde::{Serialize, Deserialize};
#[derive(Deserialize,Serialize, Debug)]
pub struct Usersign {
    pub user: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub enum Filteruser {
    Username(String),
    Email(String),
    PhoneNumber(String)
}
mod db {
    use deadpool_postgres::Client;
    use tokio_pg_mapper::FromTokioPostgresRow;

    use crate::{errors::MyError, models::User, Usersign,Filteruser};

    pub async fn get_users(client: &Client, user:Filteruser) -> Result<Vec<User>, MyError> {
        let mut stmt = include_str!("../sql/get_users.sql");
        let stmt:String = match user{
            Filteruser::Username(username) => {
                let stmt = stmt.replace("$filterrow", &"username".to_owned());
                stmt.replace("$user_sign", &username)
            },
            Filteruser::Email(email) => {
                let stmt = stmt.replace("$filterrow", &"email".to_owned());
                stmt.replace("$user_sign", &email)
                
            },
            Filteruser::PhoneNumber(phone) => {
                let stmt = stmt.replace("$filterrow", &"phone".to_owned());
                stmt.replace("$user_sign", &phone)
            },
        };
        let stmt = stmt.replace("$table_fields", &User::sql_table_fields());

      
        println!("{:?}",stmt);
        let stmt = client.prepare(&stmt).await.unwrap();
        let results = client
            .query(&stmt, &[])
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>();

        Ok(results)
    }

    pub async fn add_user(client: &Client, user_info: User) -> Result<User, MyError> {
        let _stmt = include_str!("../sql/add_user.sql");
        let _stmt = _stmt.replace("$table_fields", &User::sql_table_fields());
        let stmt = client.prepare(&_stmt).await.unwrap();

        client
            .query(
                &stmt,
                &[
                    &user_info.username,
                    &user_info.pwd,
                    &user_info.email,
                    &user_info.phone,
                ],
            )
            .await?
            .iter()
            .map(|row| User::from_row_ref(row).unwrap())
            .collect::<Vec<User>>()
            .pop()
            .ok_or(MyError::NotFound) // more applicable for SELECTs
    }
}


mod handlers {

    use serde_json;
    use actix_web::{web, Error, HttpResponse, http::{self, StatusCode, header::q}, body::BoxBody};
    use deadpool_postgres::{Client, Pool};

    use crate::{db, errors::MyError, models::User, Usersign, Filteruser};
    
    pub async fn get_users(db_pool: web::Data<Pool>, query:web::Query<Usersign>) -> Result<HttpResponse, Error> {
        let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;
        println!("client:{:?}",client);
        println!("query----:{:?}",query);
        // let mut users:Vec<User>;
        let mut filterinfo  = query.user.split('-');
        let (filter_filed, filterstr) = (filterinfo.next(),filterinfo.next());
        let filter_string = match filterstr {
            Some(st) => {st.to_string()},
            None => {"".to_owned()},
        };
        let users = match filter_filed {
            Some("username") =>{
                // let Some(filteruser) = filterinfo.next();
                db::get_users(&client,Filteruser::Username(String::from(filter_string))).await?
            },
            Some("email") =>{
                // let Some(filteremail) = filterinfo.next();
                db::get_users(&client,Filteruser::Email(filter_string.to_owned())).await?
            },
            Some("phone") =>{
                // let Some(filterphone) = filterinfo.next();
                db::get_users(&client,Filteruser::PhoneNumber(filter_string.to_owned())).await?
            },
            Some(&_) =>{
                vec![User {
                    username: "not found".to_owned(),
                    pwd: "not found".to_owned(),
                    email: "not found".to_owned(),
                    phone: "not found".to_owned(),
                }]
            },
            None =>{
                vec![User {
                    username: "not found".to_owned(),
                    pwd: "not found".to_owned(),
                    email: "not found".to_owned(),
                    phone: "not found".to_owned(),
                }]
                // return Err();
                // return Err(core::fmt::Error("not found"));
            }
        };
        // if filterinfo.next() == "username".to_owned() {
        // }else if query.user.contains("email"){
        // }else if query.user.contains("phone"){
        // }

        Ok(HttpResponse::Ok().json(users))
    }

    pub async fn add_user(
        user: web::Json<User>,
        db_pool: web::Data<Pool>,
    ) -> Result<HttpResponse, Error> {
        let user_info: User = user.into_inner();
        println!("user_info:{:?}",user_info);
        let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;
        // if let Ok(client) = match db_pool.get().await {
        //     Ok(cli) => cli,
        //     Err(error) => {
        //         // .map_err(MyError::PoolError)
        //         Err(error)
        //     },
        // };
        println!("db_pool get");

        // let new_user = db::add_user(&client, user_info).await?;
        match db::add_user(&client, user_info).await {
            Ok(new_user) => {
                println!("add_user ok,{:?}",new_user);
                Ok(HttpResponse::Ok().json(new_user))
            },
            Err(error) => {
                println!("add_user fail");
                // let error_detail :MyError;
                match &error {
                    MyError::NotFound => todo!(),
                    MyError::PGError(err) => {
                        
                        println!("PGrror:{:?}",err);
                        let resbody = BoxBody::new(err.to_string());
                        Ok(HttpResponse::with_body(StatusCode::from_u16(500).unwrap(),resbody))
                    },
                    MyError::PGMError(err) => {
                        println!("PGError:{:?}",err);
                        let resbody = BoxBody::new(err.to_string());
                        Ok(HttpResponse::with_body(StatusCode::from_u16(500).unwrap(),resbody))
                    },
                    MyError::PoolError(err) => {
                        println!("PoolError:{:?}",err);
                        let resbody = BoxBody::new(err.to_string());
                        Ok(HttpResponse::with_body(StatusCode::from_u16(500).unwrap(),resbody))
                    },
                }
                // Err(error.into())
            },
        }
        
    }
}

use ::config::Config;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use handlers::{add_user, get_users};
// use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

use crate::config::ExampleConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config_ = Config::builder()
        .add_source(::config::Environment::default())
        .build()
        .unwrap();

    let config: ExampleConfig = config_.try_deserialize().unwrap();

    let pool = config.pg.create_pool(None, NoTls).unwrap();

    let server = HttpServer::new(move || {
        App::new().app_data(web::Data::new(pool.clone())).service(
            web::resource("/users")
                .route(web::post().to(add_user))
                .route(web::get().to(get_users)),
        )
    })
    .bind(config.server_addr.clone())?
    .run();
    println!("Server running at http://{}/", config.server_addr);

    server.await
}
