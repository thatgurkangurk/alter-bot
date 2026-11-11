use std::net::SocketAddr;

use crate::{module::Module, register_module};

pub struct WebModule;

#[async_trait::async_trait]
impl Module for WebModule {
    fn name(&self) -> &'static str {
        "web"
    }

    fn depends_on(&self) -> &'static [&'static str] {
        &["database"]
    }

    async fn init(&self) -> anyhow::Result<()> {
        let web_addr = SocketAddr::from(([0, 0, 0, 0], 3000));

        tokio::spawn(async move {
            if let Err(err) = crate::web::run(web_addr).await {
                tracing::error!(error = %err, "web server stopped unexpectedly");
            }
        });

        Ok(())
    }
}

register_module!(WebModule);
