//! Language Server Protocol (LSP) implementation for Tarqeem
//!
//! This module provides full IDE support through the LSP protocol, enabling:
//! - Real-time diagnostics (أخطاء وتحذيرات)
//! - Auto-completion (إكمال تلقائي)
//! - Go to definition (انتقال للتعريف)
//! - Hover information (معلومات عند التحويم)
//! - Find references (البحث عن المراجع)
//! - Rename symbol (إعادة التسمية)
//! - Document outline (مخطط المستند)

mod analysis;
mod capabilities;
mod handlers;
mod server;
mod state;
mod utils;

pub use server::TarqeemLanguageServer;
pub use state::ServerState;

use std::error::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tower_lsp::{LspService, Server};

/// Start the LSP server on stdin/stdout
pub async fn run_server() -> Result<(), Box<dyn Error + Send + Sync>> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    run_server_with_io(stdin, stdout).await
}

/// Start the LSP server with custom I/O
pub async fn run_server_with_io<I, O>(
    input: I,
    output: O,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
{
    let (service, socket) = LspService::new(TarqeemLanguageServer::new);
    Server::new(input, output, socket).serve(service).await;
    Ok(())
}
