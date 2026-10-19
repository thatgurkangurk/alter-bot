pub mod error;
pub mod middleware;
mod routes;
mod server;

#[allow(unused_imports)]
pub use error::{AppError, AppResult};
pub use server::{AppState, WebServer};
