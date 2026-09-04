use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum EcrError {
    #[error("RepositoryAlreadyExistsException: The repository with name '{0}' already exists")]
    RepositoryAlreadyExists(String),
    #[error("RepositoryNotFoundException: The repository with name '{0}' does not exist")]
    RepositoryNotFound(String),
    #[error("ImageNotFoundException: The image does not exist")]
    ImageNotFound(String),
    #[error("InvalidParameterException: {0}")]
    InvalidParameter(String),
}

#[derive(Clone)]
pub struct EcrState {
    pub account_id: String,
    pub region: String,
    repositories: Arc<DashMap<String, Arc<RwLock<StoredRepository>>>>,
}

impl EcrState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            repositories: Arc::new(DashMap::new()),
        }
    }

    pub fn format_repository_arn(&self, name: &str) -> String {
        format!(
            "arn:aws:ecr:{}:{}:repository/{}",
            self.region, self.account_id, name
        )
    }

    pub fn format_repository_uri(&self, name: &str) -> String {
        format!(
            "{}.dkr.ecr.{}.amazonaws.com/{}",
            self.account_id, self.region, name
        )
    }

    pub fn create_repository(
        &self,
        req: CreateRepositoryRequest,
    ) -> Result<CreateRepositoryResponse, EcrError> {
        if self.repositories.contains_key(&req.repository_name) {
            return Err(EcrError::RepositoryAlreadyExists(req.repository_name));
        }

        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let repo = Repository {
            repository_arn: self.format_repository_arn(&req.repository_name),
            registry_id: self.account_id.clone(),
            repository_name: req.repository_name.clone(),
            repository_uri: self.format_repository_uri(&req.repository_name),
            created_at: now,
            image_tag_mutability: req.image_tag_mutability.or(Some("MUTABLE".to_string())),
            image_scanning_configuration: Some(serde_json::json!({"scanOnPush": false})),
            encryption_configuration: Some(serde_json::json!({"encryptionType": "AES256"})),
        };

        let stored = StoredRepository {
            repository: repo.clone(),
            images: Vec::new(),
            policy_text: None,
        };

        self.repositories
            .insert(req.repository_name, Arc::new(RwLock::new(stored)));

        Ok(CreateRepositoryResponse { repository: repo })
    }

    pub fn describe_repositories(
        &self,
        req: DescribeRepositoriesRequest,
    ) -> Result<DescribeRepositoriesResponse, EcrError> {
        let mut list = Vec::new();
        if let Some(names) = req.repository_names {
            for name in names {
                if let Some(entry) = self.repositories.get(&name) {
                    list.push(entry.read().repository.clone());
                } else {
                    return Err(EcrError::RepositoryNotFound(name));
                }
            }
        } else {
            for item in self.repositories.iter() {
                list.push(item.value().read().repository.clone());
            }
        }
        list.sort_by(|a, b| a.repository_name.cmp(&b.repository_name));
        let limit = req.max_results.unwrap_or(100);
        if list.len() > limit {
            list.truncate(limit);
        }

        Ok(DescribeRepositoriesResponse {
            repositories: list,
            next_token: None,
        })
    }

    pub fn delete_repository(
        &self,
        req: DeleteRepositoryRequest,
    ) -> Result<DeleteRepositoryResponse, EcrError> {
        let (_, entry) = self
            .repositories
            .remove(&req.repository_name)
            .ok_or_else(|| EcrError::RepositoryNotFound(req.repository_name))?;

        let repo = entry.read().repository.clone();
        Ok(DeleteRepositoryResponse { repository: repo })
    }

    pub fn get_authorization_token(&self) -> GetAuthorizationTokenResponse {
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let expires = now + 43200.0; // 12 hours
        let raw_token = format!("AWS:ruststack-token-{}", self.account_id);
        use base64::Engine;
        let b64_token = base64::engine::general_purpose::STANDARD.encode(raw_token.as_bytes());

        let proxy = format!(
            "https://{}.dkr.ecr.{}.amazonaws.com",
            self.account_id, self.region
        );

        GetAuthorizationTokenResponse {
            authorization_data: vec![AuthorizationData {
                authorization_token: b64_token,
                expires_at: expires,
                proxy_endpoint: proxy,
            }],
        }
    }

    pub fn put_image(&self, req: PutImageRequest) -> Result<PutImageResponse, EcrError> {
        let entry = self
            .repositories
            .get(&req.repository_name)
            .ok_or_else(|| EcrError::RepositoryNotFound(req.repository_name.clone()))?;

        let digest = req.image_digest.clone().unwrap_or_else(|| {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(req.image_manifest.as_bytes());
            let result = hasher.finalize();
            format!("sha256:{:x}", result)
        });

        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let stored_img = StoredImage {
            digest: digest.clone(),
            tag: req.image_tag.clone(),
            manifest: req.image_manifest.clone(),
            media_type: req.image_manifest_media_type.clone(),
            pushed_at: now,
        };

        let mut repo = entry.write();
        repo.images.push(stored_img);

        Ok(PutImageResponse {
            image: Image {
                registry_id: self.account_id.clone(),
                repository_name: req.repository_name,
                image_id: ImageIdentifier {
                    image_digest: Some(digest),
                    image_tag: req.image_tag,
                },
                image_manifest: Some(req.image_manifest),
                image_manifest_media_type: req.image_manifest_media_type,
            },
        })
    }

    pub fn batch_get_image(&self, req: BatchGetImageRequest) -> Result<BatchGetImageResponse, EcrError> {
        let entry = self
            .repositories
            .get(&req.repository_name)
            .ok_or_else(|| EcrError::RepositoryNotFound(req.repository_name.clone()))?;

        let repo = entry.read();
        let mut images = Vec::new();
        let mut failures = Vec::new();

        for target in req.image_ids {
            let found = repo.images.iter().find(|img| {
                if let Some(ref d) = target.image_digest {
                    &img.digest == d
                } else if let Some(ref t) = target.image_tag {
                    img.tag.as_ref() == Some(t)
                } else {
                    false
                }
            });

            if let Some(img) = found {
                images.push(Image {
                    registry_id: self.account_id.clone(),
                    repository_name: req.repository_name.clone(),
                    image_id: ImageIdentifier {
                        image_digest: Some(img.digest.clone()),
                        image_tag: img.tag.clone(),
                    },
                    image_manifest: Some(img.manifest.clone()),
                    image_manifest_media_type: img.media_type.clone(),
                });
            } else {
                failures.push(serde_json::json!({
                    "imageId": target,
                    "failureCode": "ImageNotFound",
                    "failureReason": "Image not found"
                }));
            }
        }

        Ok(BatchGetImageResponse { images, failures })
    }

    pub fn list_images(&self, req: ListImagesRequest) -> Result<ListImagesResponse, EcrError> {
        let entry = self
            .repositories
            .get(&req.repository_name)
            .ok_or_else(|| EcrError::RepositoryNotFound(req.repository_name))?;

        let repo = entry.read();
        let mut image_ids = Vec::new();
        for img in &repo.images {
            image_ids.push(ImageIdentifier {
                image_digest: Some(img.digest.clone()),
                image_tag: img.tag.clone(),
            });
        }
        let limit = req.max_results.unwrap_or(100);
        if image_ids.len() > limit {
            image_ids.truncate(limit);
        }

        Ok(ListImagesResponse {
            image_ids,
            next_token: None,
        })
    }

    pub fn batch_delete_image(&self, req: BatchDeleteImageRequest) -> Result<BatchDeleteImageResponse, EcrError> {
        let entry = self
            .repositories
            .get(&req.repository_name)
            .ok_or_else(|| EcrError::RepositoryNotFound(req.repository_name))?;

        let mut repo = entry.write();
        let mut deleted = Vec::new();
        let mut failures = Vec::new();

        for target in req.image_ids {
            let idx = repo.images.iter().position(|img| {
                if let Some(ref d) = target.image_digest {
                    &img.digest == d
                } else if let Some(ref t) = target.image_tag {
                    img.tag.as_ref() == Some(t)
                } else {
                    false
                }
            });

            if let Some(i) = idx {
                repo.images.remove(i);
                deleted.push(target);
            } else {
                failures.push(serde_json::json!({
                    "imageId": target,
                    "failureCode": "ImageNotFound"
                }));
            }
        }

        Ok(BatchDeleteImageResponse {
            image_ids: deleted,
            failures,
        })
    }

    pub fn export_snapshot(&self) -> EcrStateSnapshot {
        let mut map = HashMap::new();
        for item in self.repositories.iter() {
            map.insert(item.key().clone(), item.value().read().clone());
        }
        EcrStateSnapshot { repositories: map }
    }

    pub fn import_snapshot(&self, snapshot: EcrStateSnapshot) {
        self.repositories.clear();
        for (k, v) in snapshot.repositories {
            self.repositories.insert(k, Arc::new(RwLock::new(v)));
        }
    }

    pub fn reset(&self) {
        self.repositories.clear();
    }
}
