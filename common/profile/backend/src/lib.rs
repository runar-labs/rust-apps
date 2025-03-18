use anyhow::{anyhow, Result};
use async_trait::async_trait;
use auth_service::User;
use chrono::{DateTime, Utc};
use kagi_macros::{action, init, service};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[service]
pub struct ProfileService {
    profiles: Arc<RwLock<HashMap<Uuid, Profile>>>,
    user_profile_index: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

#[init]
impl ProfileService {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            user_profile_index: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl ProfileService {
    #[action]
    pub async fn create_profile(&self, user: User) -> Result<Profile> {
        let now = Utc::now();
        let profile = Profile {
            id: Uuid::new_v4(),
            user_id: user.id,
            display_name: user.username,
            bio: None,
            avatar_url: None,
            created_at: now,
            updated_at: now,
        };

        {
            let mut profiles = self.profiles.write().await;
            let mut user_profile_index = self.user_profile_index.write().await;

            if user_profile_index.contains_key(&user.id) {
                return Err(anyhow!("Profile already exists for user"));
            }

            user_profile_index.insert(user.id, profile.id);
            profiles.insert(profile.id, profile.clone());
        }

        Ok(profile)
    }

    #[action]
    pub async fn get_profile(&self, user_id: Uuid) -> Result<Profile> {
        let profile_id = {
            let user_profile_index = self.user_profile_index.read().await;
            user_profile_index
                .get(&user_id)
                .ok_or_else(|| anyhow!("Profile not found"))?
                .clone()
        };

        let profiles = self.profiles.read().await;
        profiles
            .get(&profile_id)
            .cloned()
            .ok_or_else(|| anyhow!("Profile not found"))
    }

    #[action]
    pub async fn update_profile(&self, user_id: Uuid, req: UpdateProfileRequest) -> Result<Profile> {
        let profile_id = {
            let user_profile_index = self.user_profile_index.read().await;
            user_profile_index
                .get(&user_id)
                .ok_or_else(|| anyhow!("Profile not found"))?
                .clone()
        };

        let mut profiles = self.profiles.write().await;
        let profile = profiles
            .get_mut(&profile_id)
            .ok_or_else(|| anyhow!("Profile not found"))?;

        if let Some(display_name) = req.display_name {
            profile.display_name = display_name;
        }
        if let Some(bio) = req.bio {
            profile.bio = Some(bio);
        }
        if let Some(avatar_url) = req.avatar_url {
            profile.avatar_url = Some(avatar_url);
        }
        profile.updated_at = Utc::now();

        Ok(profile.clone())
    }

    #[action]
    pub async fn delete_profile(&self, user_id: Uuid) -> Result<()> {
        let profile_id = {
            let mut user_profile_index = self.user_profile_index.write().await;
            user_profile_index
                .remove(&user_id)
                .ok_or_else(|| anyhow!("Profile not found"))?
        };

        let mut profiles = self.profiles.write().await;
        profiles
            .remove(&profile_id)
            .ok_or_else(|| anyhow!("Profile not found"))?;

        Ok(())
    }
} 