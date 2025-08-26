use axum::{
    Json,
    Router,
    routing::{get, post, put, delete},
    response::IntoResponse,
    extract::State,
};
use tokio::net::TcpListener;
use serde::{Serialize, Deserialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use dotenv::dotenv;
use std::env;


#[derive(Debug, Deserialize, Clone, Serialize)]
struct UserProfile {
    userid: i32,   
    email: String,
    password: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
struct UpdatePassword {
    password: Option<String>, 
}

type Db = Pool<Postgres>;



async fn hello_greet() -> &'static str {
    "Hello Bro"
}

async fn fetch_whole_db(State(db): State<Db>) -> impl IntoResponse {
    let users = sqlx::query_as!(UserProfile, "SELECT userid, password, email FROM users")
        .fetch_all(&db)
        .await
        .unwrap();

    Json(users)
}

async fn register_user(
    State(db): State<Db>,
    Json(payload): Json<UserProfile>
) -> impl IntoResponse {
    let rec = sqlx::query!(
        r#"
        INSERT INTO users (email, password)
        VALUES ($1, $2)
        RETURNING userid, email, password
        "#,
        payload.email,
        payload.password
    )
    .fetch_one(&db)
    .await
    .unwrap();

    let user = UserProfile {
        userid: rec.userid,
        email: rec.email,
        password: rec.password,
    };

    Json(user)
}

async fn update_user(
    axum::extract::Path(id): axum::extract::Path<i32>,
    State(db): State<Db>,
    Json(payload): Json<UpdatePassword>,
) -> impl IntoResponse {
    if let Some(new_password) = payload.password {
        let rec = sqlx::query!(
            r#"
            UPDATE users
            SET password = $1
            WHERE userid = $2
            RETURNING userid, email, password
            "#,
            new_password,
            id
        )
        .fetch_optional(&db)
        .await
        .unwrap();

        if let Some(user) = rec {
            let updated_user = UserProfile {
                userid: user.userid,
                email: user.email,
                password: user.password,
            };
            Json(serde_json::json!({
                "message": "User password updated",
                "user": updated_user
            }))
        } else {
            Json(serde_json::json!({ "error": "User not found" }))
        }
    } else {
        Json(serde_json::json!({ "error": "Password not provided" }))
    }
}

async fn delete_registered_user(
    axum::extract::Path(id): axum::extract::Path<i32>,
    State(db): State<Db>
) -> impl IntoResponse {
    let rows_affected = sqlx::query!(
        r#"
        DELETE FROM users
        WHERE userid = $1
        "#,
        id
    )
    .execute(&db)
    .await
    .unwrap()
    .rows_affected();

    if rows_affected > 0 {
        Json(serde_json::json!({ "message": "User got deleted" }))
    } else {
        Json(serde_json::json!({ "error": "User not found" }))
    }
}


fn new_router(db: Db) -> Router {
    Router::new()
        .route("/", get(hello_greet))
        .route("/fetch-whole-db", get(fetch_whole_db))
        .route("/register", post(register_user))
        .route("/update-pw/:id", put(update_user))
        .route("/delete-user/:id", delete(delete_registered_user))
        .with_state(db)
}



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok(); 

    let db_url = env::var("DATABASE_URL")?;
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    let app = new_router(db);
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("The server is running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

