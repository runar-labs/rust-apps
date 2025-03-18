use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use kagi_macros::{action, init, service};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: User,
    pub token: String,
}

const JWT_SECRET: &[u8] = b"your-secret-key"; // In production, use environment variable
const TOKEN_EXPIRATION_HOURS: i64 = 24;

#[service]
pub struct AuthService {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    username_index: Arc<RwLock<HashMap<String, Uuid>>>,
    email_index: Arc<RwLock<HashMap<String, Uuid>>>,
}

#[init]
impl AuthService {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            username_index: Arc::new(RwLock::new(HashMap::new())),
            email_index: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

impl AuthService {
    async fn create_token(&self, user_id: Uuid) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(TOKEN_EXPIRATION_HOURS);
        
        let claims = Claims {
            sub: user_id,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET),
        )
        .map_err(|e| anyhow!("Failed to create token: {}", e))
    }

    async fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|e| anyhow!("Invalid token: {}", e))?;

        Ok(token_data.claims)
    }
}

#[async_trait]
impl AuthService {
    #[action]
    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse> {
        // Validate input
        if req.password.len() < 8 {
            return Err(anyhow!("Password must be at least 8 characters long"));
        }

        // Check if username or email already exists
        {
            let username_index = self.username_index.read().await;
            let email_index = self.email_index.read().await;
            
            if username_index.contains_key(&req.username) {
                return Err(anyhow!("Username already exists"));
            }
            if email_index.contains_key(&req.email) {
                return Err(anyhow!("Email already exists"));
            }
        }

        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            username: req.username.clone(),
            email: req.email.clone(),
            password_hash: hash(req.password.as_bytes(), DEFAULT_COST)?,
            created_at: now,
            updated_at: now,
        };

        // Update indexes and store user
        {
            let mut users = self.users.write().await;
            let mut username_index = self.username_index.write().await;
            let mut email_index = self.email_index.write().await;

            username_index.insert(user.username.clone(), user.id);
            email_index.insert(user.email.clone(), user.id);
            users.insert(user.id, user.clone());
        }

        let token = self.create_token(user.id).await?;
        Ok(AuthResponse { user, token })
    }

    #[action]
    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        let user_id = {
            let username_index = self.username_index.read().await;
            username_index
                .get(&req.username)
                .ok_or_else(|| anyhow!("Invalid username or password"))?
                .clone()
        };

        let user = {
            let users = self.users.read().await;
            users
                .get(&user_id)
                .ok_or_else(|| anyhow!("User not found"))?
                .clone()
        };

        if !verify(req.password.as_bytes(), &user.password_hash)? {
            return Err(anyhow!("Invalid username or password"));
        }

        let token = self.create_token(user.id).await?;
        Ok(AuthResponse { user, token })
    }

    #[action]
    pub async fn validate_token(&self, token: String) -> Result<User> {
        let claims = self.verify_token(&token).await?;
        
        let users = self.users.read().await;
        users
            .get(&claims.sub)
            .cloned()
            .ok_or_else(|| anyhow!("User not found"))
    }

    #[action]
    pub async fn get_user(&self, user_id: Uuid) -> Result<User> {
        let users = self.users.read().await;
        users
            .get(&user_id)
            .cloned()
            .ok_or_else(|| anyhow!("User not found"))
    }
} 