use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::auth::{api_key_middleware, AuthConfig, UserContext};
use crate::error::VaultError;
use crate::metrics::{metrics_handler, record_key_export, record_operation, MetricsLayer};
use crate::vault::WalletVault;
use crate::wallet::{WalletMetadata, WalletTier};
use chrono::Utc;
use solana_sdk::signer::Signer;

#[derive(OpenApi)]
#[openapi(
    paths(
        create_wallet,
        get_wallet,
        list_wallets,
        rotate_wallet,
        deactivate_wallet,
        get_keypair,
        backup_vault,
        restore_vault,
        list_backups,
        reset_daily_limits,
    ),
    components(schemas(
        CreateWalletRequest,
        WalletResponse,
        WalletListResponse,
        KeypairResponse,
        BackupResponse,
        RestoreRequest,
        BackupListResponse,
        ErrorResponse,
        WalletTier,
    )),
    tags(
        (name = "wallets", description = "Wallet management endpoints"),
        (name = "backup", description = "Backup and restore endpoints"),
        (name = "security", description = "Security operations"),
    )
)]
pub struct ApiDoc;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateWalletRequest {
    pub tier: WalletTier,
    pub label: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct WalletResponse {
    pub id: Uuid,
    pub pubkey: String,
    pub tier: WalletTier,
    pub label: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub daily_transactions: u32,
    pub total_volume_lamports: u64,
}

impl From<WalletMetadata> for WalletResponse {
    fn from(metadata: WalletMetadata) -> Self {
        WalletResponse {
            id: metadata.id,
            pubkey: metadata.pubkey,
            tier: metadata.tier,
            label: metadata.label,
            created_at: metadata.created_at,
            is_active: metadata.is_active,
            daily_transactions: metadata.daily_transactions,
            total_volume_lamports: metadata.total_volume_lamports,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct WalletListResponse {
    pub wallets: Vec<WalletResponse>,
    pub total: usize,
}

#[derive(Serialize, ToSchema)]
pub struct KeypairResponse {
    pub pubkey: String,
    pub secret_key: String,
}

#[derive(Serialize, ToSchema)]
pub struct BackupResponse {
    pub backup_id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RestoreRequest {
    pub backup_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct BackupListResponse {
    pub backups: Vec<BackupInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct BackupInfo {
    pub backup_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub type AppState = Arc<WalletVault>;

pub fn create_api_router(vault: WalletVault) -> Router {
    let app_state = Arc::new(vault);

    // Create auth config - disable in development, enable in production
    let auth_required = std::env::var("WALLET_VAULT_AUTH_REQUIRED")
        .unwrap_or_else(|_| "false".to_string())
        == "true";
    let jwt_secret =
        std::env::var("WALLET_VAULT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());

    let auth_config = Arc::new(AuthConfig::new(jwt_secret, auth_required));

    let protected_routes = Router::new()
        .route("/api/wallets", post(create_wallet))
        .route("/api/wallets", get(list_wallets))
        .route("/api/wallets/:id", get(get_wallet))
        .route("/api/wallets/:id/rotate", put(rotate_wallet))
        .route("/api/wallets/:id/deactivate", delete(deactivate_wallet))
        .route("/api/wallets/:id/keypair", get(get_keypair))
        .route("/api/backup", post(backup_vault))
        .route("/api/backup/restore", post(restore_vault))
        .route("/api/backup/list", get(list_backups))
        .route("/api/limits/reset", post(reset_daily_limits))
        .layer(middleware::from_fn_with_state(
            auth_config.clone(),
            api_key_middleware,
        ));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_check))
        .merge(protected_routes)
        .layer(MetricsLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(app_state)
        .with_state(auth_config)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "wallet-vault",
        "timestamp": Utc::now(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/wallets",
    request_body = CreateWalletRequest,
    responses(
        (status = 200, description = "Wallet created successfully", body = WalletResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    tag = "wallets"
)]
async fn create_wallet(
    State(vault): State<AppState>,
    Extension(user_ctx): Extension<UserContext>,
    Json(request): Json<CreateWalletRequest>,
) -> impl IntoResponse {
    log::info!(
        "User {} creating wallet with tier {:?}",
        user_ctx.user_id,
        request.tier
    );

    match vault.create_wallet(request.tier, request.label).await {
        Ok(metadata) => {
            record_operation("create_wallet", true);
            Ok(Json(WalletResponse::from(metadata)))
        }
        Err(e) => {
            record_operation("create_wallet", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/wallets/{id}",
    params(
        ("id" = Uuid, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Wallet found", body = WalletResponse),
        (status = 404, description = "Wallet not found", body = ErrorResponse),
    ),
    tag = "wallets"
)]
async fn get_wallet(State(vault): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match vault.get_wallet_metadata(id).await {
        Ok(metadata) => {
            record_operation("get_wallet", true);
            Ok(Json(WalletResponse::from(metadata)))
        }
        Err(VaultError::WalletNotFound(_)) => {
            record_operation("get_wallet", false);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Wallet not found".to_string(),
                }),
            ))
        }
        Err(e) => {
            record_operation("get_wallet", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/wallets",
    responses(
        (status = 200, description = "List of wallets", body = WalletListResponse),
    ),
    tag = "wallets"
)]
async fn list_wallets(State(vault): State<AppState>) -> impl IntoResponse {
    match vault.list_wallets().await {
        Ok(wallets) => {
            record_operation("list_wallets", true);
            let total = wallets.len();
            let wallet_responses: Vec<WalletResponse> =
                wallets.into_iter().map(WalletResponse::from).collect();
            Ok(Json(WalletListResponse {
                wallets: wallet_responses,
                total,
            }))
        }
        Err(e) => {
            record_operation("list_wallets", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/wallets/{id}/rotate",
    params(
        ("id" = Uuid, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Wallet rotated successfully", body = WalletResponse),
        (status = 404, description = "Wallet not found", body = ErrorResponse),
    ),
    tag = "security"
)]
async fn rotate_wallet(State(vault): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match vault.rotate_wallet(id).await {
        Ok(metadata) => {
            record_operation("rotate_wallet", true);
            Ok(Json(WalletResponse::from(metadata)))
        }
        Err(VaultError::WalletNotFound(_)) => {
            record_operation("rotate_wallet", false);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Wallet not found".to_string(),
                }),
            ))
        }
        Err(e) => {
            record_operation("rotate_wallet", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/wallets/{id}/deactivate",
    params(
        ("id" = Uuid, Path, description = "Wallet ID")
    ),
    responses(
        (status = 204, description = "Wallet deactivated successfully"),
        (status = 404, description = "Wallet not found", body = ErrorResponse),
    ),
    tag = "wallets"
)]
async fn deactivate_wallet(
    State(vault): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match vault.deactivate_wallet(id).await {
        Ok(_) => {
            record_operation("deactivate_wallet", true);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            record_operation("deactivate_wallet", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/wallets/{id}/keypair",
    params(
        ("id" = Uuid, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Keypair retrieved", body = KeypairResponse),
        (status = 404, description = "Wallet not found", body = ErrorResponse),
        (status = 403, description = "Access denied", body = ErrorResponse),
    ),
    tag = "security"
)]
async fn get_keypair(State(vault): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match vault.get_keypair(id).await {
        Ok(keypair) => {
            record_operation("get_keypair", true);
            record_key_export(true);
            Ok(Json(KeypairResponse {
                pubkey: keypair.pubkey().to_string(),
                secret_key: bs58::encode(keypair.secret().as_bytes()).into_string(),
            }))
        }
        Err(VaultError::WalletNotFound(_)) => {
            record_operation("get_keypair", false);
            record_key_export(false);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Wallet not found".to_string(),
                }),
            ))
        }
        Err(VaultError::AccessDenied) => {
            record_operation("get_keypair", false);
            record_key_export(false);
            Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".to_string(),
                }),
            ))
        }
        Err(e) => {
            record_operation("get_keypair", false);
            record_key_export(false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/backup",
    responses(
        (status = 200, description = "Backup created successfully", body = BackupResponse),
        (status = 500, description = "Backup failed", body = ErrorResponse),
    ),
    tag = "backup"
)]
async fn backup_vault(State(vault): State<AppState>) -> impl IntoResponse {
    match vault.backup().await {
        Ok(backup_id) => {
            record_operation("backup_vault", true);
            Ok(Json(BackupResponse {
                backup_id,
                message: "Backup created successfully".to_string(),
            }))
        }
        Err(e) => {
            record_operation("backup_vault", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/backup/restore",
    request_body = RestoreRequest,
    responses(
        (status = 200, description = "Restore completed successfully", body = BackupResponse),
        (status = 400, description = "Invalid backup ID", body = ErrorResponse),
        (status = 500, description = "Restore failed", body = ErrorResponse),
    ),
    tag = "backup"
)]
async fn restore_vault(
    State(vault): State<AppState>,
    Json(request): Json<RestoreRequest>,
) -> impl IntoResponse {
    match vault.restore(&request.backup_id).await {
        Ok(count) => {
            record_operation("restore_vault", true);
            Ok(Json(BackupResponse {
                backup_id: request.backup_id,
                message: format!("Restored {} wallets successfully", count),
            }))
        }
        Err(e) => {
            record_operation("restore_vault", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/backup/list",
    responses(
        (status = 200, description = "List of backups", body = BackupListResponse),
    ),
    tag = "backup"
)]
async fn list_backups(State(vault): State<AppState>) -> impl IntoResponse {
    match vault.backup_manager.list_backups().await {
        Ok(backups) => {
            record_operation("list_backups", true);
            let backup_infos: Vec<BackupInfo> = backups
                .into_iter()
                .map(|(id, created_at)| BackupInfo {
                    backup_id: id,
                    created_at,
                })
                .collect();
            Ok(Json(BackupListResponse {
                backups: backup_infos,
            }))
        }
        Err(e) => {
            record_operation("list_backups", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/limits/reset",
    responses(
        (status = 200, description = "Daily limits reset successfully"),
    ),
    tag = "wallets"
)]
async fn reset_daily_limits(State(vault): State<AppState>) -> impl IntoResponse {
    match vault.reset_daily_limits().await {
        Ok(_) => {
            record_operation("reset_daily_limits", true);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            record_operation("reset_daily_limits", false);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}
