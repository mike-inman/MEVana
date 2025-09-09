//! Permission system for fine-grained access control
//!
//! Implements role-based access control (RBAC) with:
//! - Hierarchical permissions
//! - Resource-based authorization
//! - Dynamic permission evaluation

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use std::fmt;

/// System permissions enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // User management
    UserCreate,
    UserRead,
    UserUpdate,
    UserDelete,
    UserList,
    
    // Wallet operations
    WalletCreate,
    WalletRead,
    WalletUpdate,
    WalletDelete,
    WalletList,
    WalletExport,
    WalletImport,
    
    // Trading operations
    TradingExecute,
    TradingView,
    TradingHistory,
    TradingCancel,
    TradingPause,
    TradingResume,
    
    // Arbitrage operations
    ArbitrageView,
    ArbitrageExecute,
    ArbitrageHistory,
    ArbitrageConfig,
    ArbitrageMetrics,
    
    // API key management
    ApiKeyCreate,
    ApiKeyRead,
    ApiKeyUpdate,
    ApiKeyDelete,
    ApiKeyList,
    
    // System administration
    SystemConfig,
    SystemMetrics,
    SystemHealth,
    SystemMaintenance,
    SystemAudit,
    
    // Analytics and reporting
    AnalyticsView,
    AnalyticsExport,
    AnalyticsCustom,
    ReportGenerate,
    ReportView,
    
    // Session management
    SessionView,
    SessionTerminate,
    SessionList,
    
    // Super admin permissions
    AdminFullAccess,
    AdminUserManagement,
    AdminSystemControl,
}

impl Permission {
    /// Get all permissions as array
    pub fn all() -> &'static [Permission] {
        use Permission::*;
        &[
            UserCreate, UserRead, UserUpdate, UserDelete, UserList,
            WalletCreate, WalletRead, WalletUpdate, WalletDelete, WalletList, WalletExport, WalletImport,
            TradingExecute, TradingView, TradingHistory, TradingCancel, TradingPause, TradingResume,
            ArbitrageView, ArbitrageExecute, ArbitrageHistory, ArbitrageConfig, ArbitrageMetrics,
            ApiKeyCreate, ApiKeyRead, ApiKeyUpdate, ApiKeyDelete, ApiKeyList,
            SystemConfig, SystemMetrics, SystemHealth, SystemMaintenance, SystemAudit,
            AnalyticsView, AnalyticsExport, AnalyticsCustom, ReportGenerate, ReportView,
            SessionView, SessionTerminate, SessionList,
            AdminFullAccess, AdminUserManagement, AdminSystemControl,
        ]
    }
    
    /// Get permission category
    pub fn category(&self) -> PermissionCategory {
        use Permission::*;
        match self {
            UserCreate | UserRead | UserUpdate | UserDelete | UserList => PermissionCategory::User,
            WalletCreate | WalletRead | WalletUpdate | WalletDelete | WalletList | WalletExport | WalletImport => PermissionCategory::Wallet,
            TradingExecute | TradingView | TradingHistory | TradingCancel | TradingPause | TradingResume => PermissionCategory::Trading,
            ArbitrageView | ArbitrageExecute | ArbitrageHistory | ArbitrageConfig | ArbitrageMetrics => PermissionCategory::Arbitrage,
            ApiKeyCreate | ApiKeyRead | ApiKeyUpdate | ApiKeyDelete | ApiKeyList => PermissionCategory::ApiKey,
            SystemConfig | SystemMetrics | SystemHealth | SystemMaintenance | SystemAudit => PermissionCategory::System,
            AnalyticsView | AnalyticsExport | AnalyticsCustom | ReportGenerate | ReportView => PermissionCategory::Analytics,
            SessionView | SessionTerminate | SessionList => PermissionCategory::Session,
            AdminFullAccess | AdminUserManagement | AdminSystemControl => PermissionCategory::Admin,
        }
    }
    
    /// Get permission description
    pub fn description(&self) -> &'static str {
        use Permission::*;
        match self {
            UserCreate => "Create new users",
            UserRead => "View user information",
            UserUpdate => "Modify user information",
            UserDelete => "Delete users",
            UserList => "List all users",
            
            WalletCreate => "Create new wallets",
            WalletRead => "View wallet information",
            WalletUpdate => "Modify wallet settings",
            WalletDelete => "Delete wallets",
            WalletList => "List all wallets",
            WalletExport => "Export wallet data",
            WalletImport => "Import wallet data",
            
            TradingExecute => "Execute trades",
            TradingView => "View trading data",
            TradingHistory => "Access trading history",
            TradingCancel => "Cancel pending trades",
            TradingPause => "Pause trading operations",
            TradingResume => "Resume trading operations",
            
            ArbitrageView => "View arbitrage opportunities",
            ArbitrageExecute => "Execute arbitrage trades",
            ArbitrageHistory => "Access arbitrage history",
            ArbitrageConfig => "Configure arbitrage settings",
            ArbitrageMetrics => "View arbitrage metrics",
            
            ApiKeyCreate => "Create API keys",
            ApiKeyRead => "View API key information",
            ApiKeyUpdate => "Modify API keys",
            ApiKeyDelete => "Delete API keys",
            ApiKeyList => "List all API keys",
            
            SystemConfig => "Modify system configuration",
            SystemMetrics => "View system metrics",
            SystemHealth => "Check system health",
            SystemMaintenance => "Perform system maintenance",
            SystemAudit => "Access audit logs",
            
            AnalyticsView => "View analytics dashboards",
            AnalyticsExport => "Export analytics data",
            AnalyticsCustom => "Create custom analytics",
            ReportGenerate => "Generate reports",
            ReportView => "View reports",
            
            SessionView => "View session information",
            SessionTerminate => "Terminate user sessions",
            SessionList => "List active sessions",
            
            AdminFullAccess => "Full administrative access",
            AdminUserManagement => "Manage user accounts",
            AdminSystemControl => "Control system operations",
        }
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        use Permission::*;
        match self {
            UserCreate => "user:create",
            UserRead => "user:read",
            UserUpdate => "user:update",
            UserDelete => "user:delete",
            UserList => "user:list",
            
            WalletCreate => "wallet:create",
            WalletRead => "wallet:read",
            WalletUpdate => "wallet:update",
            WalletDelete => "wallet:delete",
            WalletList => "wallet:list",
            WalletExport => "wallet:export",
            WalletImport => "wallet:import",
            
            TradingExecute => "trading:execute",
            TradingView => "trading:view",
            TradingHistory => "trading:history",
            TradingCancel => "trading:cancel",
            TradingPause => "trading:pause",
            TradingResume => "trading:resume",
            
            ArbitrageView => "arbitrage:view",
            ArbitrageExecute => "arbitrage:execute",
            ArbitrageHistory => "arbitrage:history",
            ArbitrageConfig => "arbitrage:config",
            ArbitrageMetrics => "arbitrage:metrics",
            
            ApiKeyCreate => "apikey:create",
            ApiKeyRead => "apikey:read",
            ApiKeyUpdate => "apikey:update",
            ApiKeyDelete => "apikey:delete",
            ApiKeyList => "apikey:list",
            
            SystemConfig => "system:config",
            SystemMetrics => "system:metrics",
            SystemHealth => "system:health",
            SystemMaintenance => "system:maintenance",
            SystemAudit => "system:audit",
            
            AnalyticsView => "analytics:view",
            AnalyticsExport => "analytics:export",
            AnalyticsCustom => "analytics:custom",
            ReportGenerate => "report:generate",
            ReportView => "report:view",
            
            SessionView => "session:view",
            SessionTerminate => "session:terminate",
            SessionList => "session:list",
            
            AdminFullAccess => "admin:full",
            AdminUserManagement => "admin:users",
            AdminSystemControl => "admin:system",
        }
    }
    
    /// Parse permission from string
    pub fn from_str(s: &str) -> Option<Self> {
        use Permission::*;
        match s {
            "user:create" => Some(UserCreate),
            "user:read" => Some(UserRead),
            "user:update" => Some(UserUpdate),
            "user:delete" => Some(UserDelete),
            "user:list" => Some(UserList),
            
            "wallet:create" => Some(WalletCreate),
            "wallet:read" => Some(WalletRead),
            "wallet:update" => Some(WalletUpdate),
            "wallet:delete" => Some(WalletDelete),
            "wallet:list" => Some(WalletList),
            "wallet:export" => Some(WalletExport),
            "wallet:import" => Some(WalletImport),
            
            "trading:execute" => Some(TradingExecute),
            "trading:view" => Some(TradingView),
            "trading:history" => Some(TradingHistory),
            "trading:cancel" => Some(TradingCancel),
            "trading:pause" => Some(TradingPause),
            "trading:resume" => Some(TradingResume),
            
            "arbitrage:view" => Some(ArbitrageView),
            "arbitrage:execute" => Some(ArbitrageExecute),
            "arbitrage:history" => Some(ArbitrageHistory),
            "arbitrage:config" => Some(ArbitrageConfig),
            "arbitrage:metrics" => Some(ArbitrageMetrics),
            
            "apikey:create" => Some(ApiKeyCreate),
            "apikey:read" => Some(ApiKeyRead),
            "apikey:update" => Some(ApiKeyUpdate),
            "apikey:delete" => Some(ApiKeyDelete),
            "apikey:list" => Some(ApiKeyList),
            
            "system:config" => Some(SystemConfig),
            "system:metrics" => Some(SystemMetrics),
            "system:health" => Some(SystemHealth),
            "system:maintenance" => Some(SystemMaintenance),
            "system:audit" => Some(SystemAudit),
            
            "analytics:view" => Some(AnalyticsView),
            "analytics:export" => Some(AnalyticsExport),
            "analytics:custom" => Some(AnalyticsCustom),
            "report:generate" => Some(ReportGenerate),
            "report:view" => Some(ReportView),
            
            "session:view" => Some(SessionView),
            "session:terminate" => Some(SessionTerminate),
            "session:list" => Some(SessionList),
            
            "admin:full" => Some(AdminFullAccess),
            "admin:users" => Some(AdminUserManagement),
            "admin:system" => Some(AdminSystemControl),
            
            _ => None,
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Permission categories for organization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionCategory {
    User,
    Wallet,
    Trading,
    Arbitrage,
    ApiKey,
    System,
    Analytics,
    Session,
    Admin,
}

/// Set of permissions with efficient operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    permissions: HashSet<Permission>,
}

impl PermissionSet {
    /// Create empty permission set
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
        }
    }
    
    /// Create permission set from permissions
    pub fn from_permissions(permissions: &[Permission]) -> Self {
        Self {
            permissions: permissions.iter().cloned().collect(),
        }
    }
    
    /// Create permission set from string representations
    pub fn from_strings(strings: &[String]) -> Self {
        let permissions = strings
            .iter()
            .filter_map(|s| Permission::from_str(s))
            .collect();
        
        Self { permissions }
    }
    
    /// Add permission to set
    pub fn add(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }
    
    /// Remove permission from set
    pub fn remove(&mut self, permission: Permission) {
        self.permissions.remove(&permission);
    }
    
    /// Check if permission exists in set
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission) || self.permissions.contains(&Permission::AdminFullAccess)
    }
    
    /// Get all permissions as strings
    pub fn to_strings(&self) -> Vec<String> {
        self.permissions.iter().map(|p| p.as_str().to_string()).collect()
    }
    
    /// Count permissions
    pub fn count(&self) -> usize {
        self.permissions.len()
    }
    
    /// Check if set is empty
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }
    
    /// Merge with another permission set
    pub fn merge(&mut self, other: &PermissionSet) {
        self.permissions.extend(other.permissions.iter());
    }
    
    /// Get permissions by category
    pub fn by_category(&self, category: PermissionCategory) -> Vec<Permission> {
        self.permissions
            .iter()
            .filter(|p| p.category() == category)
            .cloned()
            .collect()
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Role definition with associated permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub permissions: PermissionSet,
    pub inherits_from: Vec<String>,
}

impl Role {
    /// Create new role
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            permissions: PermissionSet::new(),
            inherits_from: Vec::new(),
        }
    }
    
    /// Add permission to role
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.add(permission);
    }
    
    /// Add parent role for inheritance
    pub fn add_parent(&mut self, parent: String) {
        self.inherits_from.push(parent);
    }
}

/// Role manager for handling role-based permissions
pub struct RoleManager {
    roles: HashMap<String, Role>,
}

impl RoleManager {
    /// Create new role manager with default roles
    pub fn new() -> Self {
        let mut manager = Self {
            roles: HashMap::new(),
        };
        
        manager.setup_default_roles();
        manager
    }
    
    /// Setup default system roles
    fn setup_default_roles(&mut self) {
        // Free user role
        let mut free_user = Role::new(
            "user".to_string(),
            "Basic user with limited permissions".to_string(),
        );
        free_user.add_permission(Permission::UserRead);
        free_user.add_permission(Permission::WalletRead);
        free_user.add_permission(Permission::TradingView);
        free_user.add_permission(Permission::ArbitrageView);
        self.add_role(free_user);
        
        // Premium user role
        let mut premium_user = Role::new(
            "premium".to_string(),
            "Premium user with enhanced permissions".to_string(),
        );
        premium_user.add_parent("user".to_string());
        premium_user.add_permission(Permission::TradingExecute);
        premium_user.add_permission(Permission::ApiKeyCreate);
        premium_user.add_permission(Permission::ApiKeyRead);
        premium_user.add_permission(Permission::AnalyticsView);
        self.add_role(premium_user);
        
        // Professional user role
        let mut professional = Role::new(
            "professional".to_string(),
            "Professional trader with advanced permissions".to_string(),
        );
        professional.add_parent("premium".to_string());
        professional.add_permission(Permission::ArbitrageExecute);
        professional.add_permission(Permission::AnalyticsExport);
        professional.add_permission(Permission::ReportGenerate);
        self.add_role(professional);
        
        // Admin role
        let mut admin = Role::new(
            "admin".to_string(),
            "System administrator with full access".to_string(),
        );
        admin.add_permission(Permission::AdminFullAccess);
        self.add_role(admin);
    }
    
    /// Add role to manager
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }
    
    /// Get role by name
    pub fn get_role(&self, name: &str) -> Option<&Role> {
        self.roles.get(name)
    }
    
    /// Get effective permissions for roles (including inheritance)
    pub fn get_effective_permissions(&self, role_names: &[String]) -> PermissionSet {
        let mut effective_permissions = PermissionSet::new();
        let mut visited = HashSet::new();
        
        for role_name in role_names {
            self.collect_permissions(role_name, &mut effective_permissions, &mut visited);
        }
        
        effective_permissions
    }
    
    /// Recursively collect permissions from role hierarchy
    fn collect_permissions(
        &self,
        role_name: &str,
        permissions: &mut PermissionSet,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(role_name) {
            return; // Prevent infinite recursion
        }
        
        visited.insert(role_name.to_string());
        
        if let Some(role) = self.roles.get(role_name) {
            permissions.merge(&role.permissions);
            
            for parent_role in &role.inherits_from {
                self.collect_permissions(parent_role, permissions, visited);
            }
        }
    }
}

impl Default for RoleManager {
    fn default() -> Self {
        Self::new()
    }
}