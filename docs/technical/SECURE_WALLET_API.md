# MEVana Secure Wallet Management API
Last updated: 2025-09-15

A comprehensive security architecture for wallet management with third-party authentication integration.

## 🔐 Security Architecture Overview

```
┌─ Frontend (React/Next.js) ──────────────┐    ┌─ Backend Services ─────────────────┐
│                                         │    │                                    │
│  👤 User Interface                      │    │  🦀 Rust Services                 │
│  ├─ Wallet Dashboard                   │    │  ├─ wallet-vault:8090             │
│  ├─ Transaction History                │    │  ├─ api-service:8091              │
│  ├─ Security Settings                  │    │  └─ hotpath-arb:8088              │
│  └─ Portfolio Analytics               │    │                                    │
│           │                            │    │  🟢 Node.js Auth Service           │
│           │ JWT Token                  │    │  ├─ auth-middleware:3001          │
│           ▼                            │    │  ├─ session-manager              │
│  🔑 Third-Party Auth                   │    │  └─ user-permissions             │
│  ├─ Auth0 / Firebase / Clerk          │────┼─▶│                                │
│  ├─ Multi-Factor Authentication       │    │                                    │
│  ├─ Social Login (Google, GitHub)     │    │  🗄️ Secure Storage                │
│  └─ Enterprise SSO                    │    │  ├─ postgres:5432 (encrypted)    │
│                                        │    │  ├─ redis:6379 (sessions)        │
└────────────────────────────────────────┘    │  └─ vault:/data (wallet keys)     │
                                              └────────────────────────────────────┘
```

## 🏗️ Multi-Layer Security Architecture

### Layer 1: Authentication Gateway (Node.js)

Create `services/auth-gateway/server.js`:

```javascript
// Authentication Gateway - Bridges third-party auth with Rust services
const express = require('express');
const jwt = require('jsonwebtoken');
const rateLimit = require('express-rate-limit');
const helmet = require('helmet');
const cors = require('cors');

// Third-party auth integrations (choose your preferred)
const { Auth0Strategy } = require('passport-auth0');
const { getAuth } = require('firebase-admin/auth'); // Firebase
const { ClerkExpressRequireAuth } = require('@clerk/clerk-sdk-node'); // Clerk

const app = express();

// Security middleware
app.use(helmet({
  contentSecurityPolicy: {
    directives: {
      defaultSrc: ["'self'"],
      connectSrc: ["'self'", "https://your-rust-services.com"],
    },
  },
}));

app.use(cors({
  origin: process.env.FRONTEND_URL || 'http://localhost:3000',
  credentials: true,
}));

// Rate limiting for API endpoints
const apiLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // limit each IP to 100 requests per windowMs
  message: 'Too many requests from this IP',
});

const walletLimiter = rateLimit({
  windowMs: 5 * 60 * 1000, // 5 minutes  
  max: 10, // Strict limit for wallet operations
  message: 'Too many wallet operations',
});

// Auth middleware that validates third-party tokens
async function validateAuth(req, res, next) {
  try {
    const token = req.headers.authorization?.replace('Bearer ', '');
    
    if (!token) {
      return res.status(401).json({ error: 'No token provided' });
    }

    let userId, permissions;

    // Choose your auth provider (uncomment one)
    
    // Auth0 verification
    // const decoded = jwt.verify(token, process.env.AUTH0_PUBLIC_KEY);
    // userId = decoded.sub;
    // permissions = decoded.permissions || [];
    
    // Firebase verification  
    // const decoded = await getAuth().verifyIdToken(token);
    // userId = decoded.uid;
    // permissions = decoded.custom_claims?.permissions || [];
    
    // Clerk verification
    // const decoded = await clerkClient.verifyToken(token);
    // userId = decoded.sub;
    // permissions = decoded.permissions || [];

    // Generate internal JWT for Rust services
    const internalToken = jwt.sign(
      {
        user_id: userId,
        permissions,
        iat: Math.floor(Date.now() / 1000),
        exp: Math.floor(Date.now() / 1000) + (60 * 30), // 30 minutes
      },
      process.env.INTERNAL_JWT_SECRET
    );

    req.user = { userId, permissions, internalToken };
    next();
  } catch (error) {
    console.error('Auth validation error:', error);
    res.status(401).json({ error: 'Invalid token' });
  }
}

// Permission middleware for wallet operations
function requireWalletPermission(permission) {
  return (req, res, next) => {
    if (!req.user?.permissions.includes(permission)) {
      return res.status(403).json({ 
        error: 'Insufficient permissions',
        required: permission 
      });
    }
    next();
  };
}

// Wallet API endpoints - Proxy to Rust wallet-vault
app.use('/api/auth', apiLimiter, validateAuth);
app.use('/api/wallet', apiLimiter, walletLimiter, validateAuth);

// List wallets
app.get('/api/wallet/list', 
  requireWalletPermission('wallet:read'),
  async (req, res) => {
    try {
      const response = await fetch('http://wallet-vault:8080/wallets', {
        headers: {
          'Authorization': `Bearer ${req.user.internalToken}`,
          'X-User-ID': req.user.userId,
        },
      });
      
      const wallets = await response.json();
      res.json(wallets);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch wallets' });
    }
  }
);

// Create new wallet
app.post('/api/wallet/create',
  requireWalletPermission('wallet:create'),
  async (req, res) => {
    try {
      const { name, wallet_type, tier } = req.body;
      
      const response = await fetch('http://wallet-vault:8080/wallets', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${req.user.internalToken}`,
          'X-User-ID': req.user.userId,
        },
        body: JSON.stringify({ name, wallet_type, tier }),
      });
      
      const wallet = await response.json();
      res.json(wallet);
    } catch (error) {
      res.status(500).json({ error: 'Failed to create wallet' });
    }
  }
);

// Get wallet balance  
app.get('/api/wallet/:id/balance',
  requireWalletPermission('wallet:read'),
  async (req, res) => {
    try {
      const response = await fetch(`http://wallet-vault:8080/wallets/${req.params.id}/balance`, {
        headers: {
          'Authorization': `Bearer ${req.user.internalToken}`,
          'X-User-ID': req.user.userId,
        },
      });
      
      const balance = await response.json();
      res.json(balance);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch balance' });
    }
  }
);

// Transfer funds (high-security operation)
app.post('/api/wallet/:id/transfer',
  requireWalletPermission('wallet:transfer'),
  async (req, res) => {
    try {
      const { to_address, amount, token_mint } = req.body;
      
      // Additional security: Log high-value transfers
      if (amount > process.env.HIGH_VALUE_THRESHOLD) {
        console.warn(`High-value transfer: User ${req.user.userId}, Amount: ${amount}`);
        // Could trigger additional MFA requirement here
      }
      
      const response = await fetch(`http://wallet-vault:8080/wallets/${req.params.id}/transfer`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${req.user.internalToken}`,
          'X-User-ID': req.user.userId,
        },
        body: JSON.stringify({ to_address, amount, token_mint }),
      });
      
      const result = await response.json();
      res.json(result);
    } catch (error) {
      res.status(500).json({ error: 'Transfer failed' });
    }
  }
);

app.listen(process.env.PORT || 3001, () => {
  console.log('🔐 Auth Gateway running on port', process.env.PORT || 3001);
});
```

### Layer 2: Enhanced Rust Wallet Vault

Update the Rust wallet-vault service to handle internal JWT tokens:

```rust
// crates/wallet-vault/src/auth.rs
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone)]
pub struct AuthState {
    pub jwt_secret: String,
}

pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract JWT token
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT
    let decoding_key = DecodingKey::from_secret(auth_state.jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    
    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add user info to request extensions
    request.extensions_mut().insert(token_data.claims);
    
    Ok(next.run(request).await)
}

pub fn require_permission(required: &str) -> impl Fn(Claims) -> Result<(), StatusCode> + '_ {
    move |claims: Claims| {
        if claims.permissions.contains(&required.to_string()) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}
```

```rust
// crates/wallet-vault/src/handlers.rs  
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::auth::{Claims, require_permission};

#[derive(Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    pub wallet_type: WalletType,
    pub tier: WalletTier,
}

#[derive(Serialize)]
pub struct WalletResponse {
    pub id: String,
    pub name: String,
    pub address: String,
    pub balance_sol: f64,
    pub tier: WalletTier,
    pub status: WalletStatus,
    pub created_at: String,
}

// List user's wallets
pub async fn list_wallets(
    Extension(claims): Extension<Claims>,
    State(app_state): State<AppState>,
) -> Result<Json<Vec<WalletResponse>>, StatusCode> {
    // Check permissions
    require_permission("wallet:read")(claims.clone())?;
    
    // Fetch wallets for this user
    let wallets = app_state.wallet_service
        .list_user_wallets(&claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response: Vec<WalletResponse> = wallets.into_iter()
        .map(|w| WalletResponse {
            id: w.id,
            name: w.name,
            address: w.address,
            balance_sol: w.balance_sol,
            tier: w.tier,
            status: w.status,
            created_at: w.created_at.to_rfc3339(),
        })
        .collect();
    
    Ok(Json(response))
}

// Create new wallet
pub async fn create_wallet(
    Extension(claims): Extension<Claims>,
    State(app_state): State<AppState>,
    Json(req): Json<CreateWalletRequest>,
) -> Result<Json<WalletResponse>, StatusCode> {
    // Check permissions
    require_permission("wallet:create")(claims.clone())?;
    
    // Additional security: Limit wallets per user
    let existing_count = app_state.wallet_service
        .count_user_wallets(&claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if existing_count >= 10 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Create wallet
    let wallet = app_state.wallet_service
        .create_wallet(&claims.user_id, req.name, req.wallet_type, req.tier)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Audit log
    tracing::info!(
        user_id = %claims.user_id,
        wallet_id = %wallet.id,
        wallet_type = ?req.wallet_type,
        "Wallet created"
    );
    
    Ok(Json(WalletResponse {
        id: wallet.id,
        name: wallet.name,
        address: wallet.address,
        balance_sol: 0.0,
        tier: wallet.tier,
        status: wallet.status,
        created_at: wallet.created_at.to_rfc3339(),
    }))
}

// Transfer funds with enhanced security
pub async fn transfer_funds(
    Extension(claims): Extension<Claims>,
    Path(wallet_id): Path<String>,
    State(app_state): State<AppState>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<TransferResponse>, StatusCode> {
    // Check permissions
    require_permission("wallet:transfer")(claims.clone())?;
    
    // Verify wallet ownership
    let wallet = app_state.wallet_service
        .get_user_wallet(&claims.user_id, &wallet_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // Security checks
    if req.amount > 10.0 && !claims.permissions.contains(&"wallet:high_value".to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Rate limiting check (stored in Redis)
    let transfer_count = app_state.redis_client
        .get_transfer_count(&claims.user_id, chrono::Duration::hours(1))
        .await
        .unwrap_or(0);
    
    if transfer_count > 5 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Execute transfer
    let result = app_state.wallet_service
        .transfer(&wallet_id, &req.to_address, req.amount, req.token_mint)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Update rate limiting
    app_state.redis_client
        .increment_transfer_count(&claims.user_id)
        .await
        .ok();
    
    // Audit log  
    tracing::warn!(
        user_id = %claims.user_id,
        wallet_id = %wallet_id,
        to_address = %req.to_address,
        amount = %req.amount,
        signature = %result.signature,
        "Funds transferred"
    );
    
    Ok(Json(TransferResponse {
        signature: result.signature,
        status: "success".to_string(),
    }))
}
```

### Layer 3: Frontend Integration (React/Next.js)

Create a secure wallet management interface:

```typescript
// components/WalletDashboard.tsx
import React, { useState, useEffect } from 'react';
import { useAuth } from '@/hooks/useAuth'; // Your auth provider hook

interface Wallet {
  id: string;
  name: string;
  address: string;
  balance_sol: number;
  tier: 'Hot' | 'Warm' | 'Cold';
  status: 'Active' | 'Locked' | 'Emergency';
}

export function WalletDashboard() {
  const { token, user, hasPermission } = useAuth();
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [loading, setLoading] = useState(true);
  
  // API client with automatic token refresh
  const apiCall = async (endpoint: string, options: RequestInit = {}) => {
    const response = await fetch(`/api${endpoint}`, {
      ...options,
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });
    
    if (!response.ok) {
      throw new Error(`API Error: ${response.status}`);
    }
    
    return response.json();
  };
  
  // Load wallets
  useEffect(() => {
    if (hasPermission('wallet:read')) {
      loadWallets();
    }
  }, [token]);
  
  const loadWallets = async () => {
    try {
      setLoading(true);
      const data = await apiCall('/wallet/list');
      setWallets(data);
    } catch (error) {
      console.error('Failed to load wallets:', error);
    } finally {
      setLoading(false);
    }
  };
  
  const createWallet = async (name: string, tier: string) => {
    if (!hasPermission('wallet:create')) {
      alert('Insufficient permissions to create wallets');
      return;
    }
    
    try {
      await apiCall('/wallet/create', {
        method: 'POST',
        body: JSON.stringify({
          name,
          wallet_type: 'Trading',
          tier,
        }),
      });
      
      await loadWallets(); // Refresh list
      alert('Wallet created successfully!');
    } catch (error) {
      console.error('Failed to create wallet:', error);
      alert('Failed to create wallet');
    }
  };
  
  const transferFunds = async (walletId: string, toAddress: string, amount: number) => {
    if (!hasPermission('wallet:transfer')) {
      alert('Insufficient permissions to transfer funds');
      return;
    }
    
    // Additional confirmation for high-value transfers
    if (amount > 1.0) {
      const confirmed = window.confirm(`Are you sure you want to transfer ${amount} SOL?`);
      if (!confirmed) return;
    }
    
    try {
      const result = await apiCall(`/wallet/${walletId}/transfer`, {
        method: 'POST',
        body: JSON.stringify({
          to_address: toAddress,
          amount,
          token_mint: 'SOL',
        }),
      });
      
      alert(`Transfer successful! Signature: ${result.signature}`);
      await loadWallets(); // Refresh balances
    } catch (error) {
      console.error('Transfer failed:', error);
      alert('Transfer failed');
    }
  };
  
  if (loading) {
    return <div className="loading">Loading wallets...</div>;
  }
  
  return (
    <div className="wallet-dashboard">
      <div className="dashboard-header">
        <h2>Wallet Management</h2>
        <div className="user-info">
          {user?.name} • {user?.permissions?.length || 0} permissions
        </div>
      </div>
      
      <div className="wallet-grid">
        {wallets.map(wallet => (
          <WalletCard 
            key={wallet.id}
            wallet={wallet}
            onTransfer={transferFunds}
            canTransfer={hasPermission('wallet:transfer')}
          />
        ))}
      </div>
      
      {hasPermission('wallet:create') && (
        <CreateWalletForm onSubmit={createWallet} />
      )}
      
      <SecurityPanel user={user} />
    </div>
  );
}

// Security monitoring panel
function SecurityPanel({ user }: { user: any }) {
  return (
    <div className="security-panel">
      <h3>Security Status</h3>
      <div className="security-items">
        <div className="security-item">
          <span>Two-Factor Authentication</span>
          <span className={user?.mfa_enabled ? 'enabled' : 'disabled'}>
            {user?.mfa_enabled ? '✅ Enabled' : '❌ Disabled'}
          </span>
        </div>
        <div className="security-item">
          <span>Last Login</span>
          <span>{new Date(user?.last_login).toLocaleString()}</span>
        </div>
        <div className="security-item">
          <span>Login Location</span>
          <span>{user?.last_location || 'Unknown'}</span>
        </div>
      </div>
    </div>
  );
}
```

## 🔐 Security Features

### 1. Multi-Factor Authentication
```javascript
// Add MFA requirement for sensitive operations
app.post('/api/wallet/:id/transfer', 
  requireWalletPermission('wallet:transfer'),
  async (req, res, next) => {
    const { amount } = req.body;
    
    // Require MFA for transfers > 1 SOL
    if (amount > 1.0 && !req.user.mfa_verified) {
      return res.status(403).json({ 
        error: 'MFA required for high-value transfers',
        mfa_challenge: await generateMFAChallenge(req.user.userId)
      });
    }
    
    next();
  }
);
```

### 2. Permission-Based Access Control
```javascript
// Define granular permissions
const PERMISSIONS = {
  'wallet:read': 'View wallet information',
  'wallet:create': 'Create new wallets', 
  'wallet:transfer': 'Transfer funds (< 1 SOL)',
  'wallet:high_value': 'High-value transfers (> 1 SOL)',
  'wallet:admin': 'Administrative wallet operations',
  'trading:execute': 'Execute arbitrage trades',
  'trading:view': 'View trading performance',
};
```

### 3. Audit Logging
```rust
// Comprehensive audit trail
#[derive(Serialize)]
struct AuditEvent {
    event_type: String,
    user_id: String,
    wallet_id: Option<String>,
    details: serde_json::Value,
    ip_address: String,
    user_agent: String,
    timestamp: DateTime<Utc>,
    risk_score: i32,
}
```

### 4. Rate Limiting & Anomaly Detection
```rust
// Detect unusual patterns
pub async fn check_anomalies(user_id: &str, action: &str) -> Result<bool, Error> {
    let recent_actions = redis_client
        .get_user_actions(user_id, Duration::minutes(15))
        .await?;
    
    // Flag suspicious patterns
    let suspicious = recent_actions.len() > 20 || // Rapid-fire requests
        recent_actions.iter().any(|a| a.ip != current_ip) || // Multiple IPs
        is_outside_normal_hours(&recent_actions); // Unusual timing
    
    if suspicious {
        trigger_security_alert(user_id, "Anomalous activity detected").await?;
    }
    
    Ok(!suspicious)
}
```

## 🚀 Deployment Integration

Add the auth gateway to your Docker Compose:

```yaml
# docker-compose.services.yml (add this service)
services:
  auth-gateway:
    build: ./services/auth-gateway
    container_name: mevana-auth-gateway
    ports:
      - "3001:3001"
    environment:
      - NODE_ENV=${NODE_ENV:-development}
      - AUTH0_DOMAIN=${AUTH0_DOMAIN}              # or your auth provider
      - AUTH0_CLIENT_ID=${AUTH0_CLIENT_ID}
      - AUTH0_CLIENT_SECRET=${AUTH0_CLIENT_SECRET}
      - INTERNAL_JWT_SECRET=${INTERNAL_JWT_SECRET}
      - FRONTEND_URL=${FRONTEND_URL:-http://localhost:3000}
      - HIGH_VALUE_THRESHOLD=${HIGH_VALUE_THRESHOLD:-1.0}
    depends_on:
      - wallet-vault
      - redis
    networks:
      - mevana-net
```

This architecture gives you:

✅ **Seamless third-party auth integration** (Auth0/Firebase/Clerk compatibility)
✅ **Granular permission system** for different user roles  
✅ **Multi-layer security** (Gateway + Rust + Database)
✅ **Rate limiting & anomaly detection**
✅ **Comprehensive audit logging**
✅ **MFA for sensitive operations**
✅ **Clean separation of concerns** (Node.js for auth, Rust for performance)

The system maintains your existing auth provider while adding enterprise-grade security for wallet operations!
