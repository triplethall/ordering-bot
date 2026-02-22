use anyhow::{Context, Result};
use grammers_client::{
    types::{Media, Peer},
    Client,
};
use grammers_tl_types as tl;
use std::path::Path;
use tokio_postgres::Client as PgClient;

/// Кэшированное изображение из БД (локальная структура)
#[derive(Debug, Clone)]
struct CachedImage {
    name: String,
    subfolder: String,
    file_id: i64,
    access_hash: i64,
    file_reference: Vec<u8>, // BYTEA из БД → Vec<u8>
}

impl CachedImage {
    /// Конвертирует в InputPhoto для отправки через grammers
    fn to_input_photo(&self) -> tl::types::InputPhoto {
        tl::types::InputPhoto {
            id: self.file_id,
            access_hash: self.access_hash,
            file_reference: self.file_reference.clone(),
        }
    }
}

/// Получает кэшированное изображение или создаёт новый кэш
pub async fn get_cached_image(
    client: &Client,
    db: &PgClient,
    cache_folder: &Path,
    subfolder: &str,
    name: &str,
    channel_id: i64,
) -> Result<tl::types::InputPhoto> {
    // 1. Проверяем кэш в БД
    if let Some(cached) = get_from_db(db, name).await? {
        return Ok(cached.to_input_photo());
    }

    // 2. Загружаем файл с диска
    let image_path = if subfolder.is_empty() {
        cache_folder.join(format!("{}.png", name))
    } else {
        cache_folder.join(subfolder).join(format!("{}.png", name))
    };

    if !image_path.exists() {
        bail!("Файл не найден: {}", image_path.display());
    }

    let file_data = tokio::fs::read(&image_path)
        .await
        .context(format!("Ошибка чтения файла {}", image_path.display()))?;

    // 3. Получаем канал по ID
    let peer = Peer::Channel(channel_id);
    let channel = client
        .get_entity(peer)
        .await
        .with_context(|| format!("Не удалось найти канал ID={}", channel_id))?;

    // 4. Загружаем и отправляем в канал
    let input_file = client
        .upload_file(&file_data)
        .await
        .context("Ошибка загрузки файла в Telegram")?;

    let msg = client
        .send_media(channel, Media::UploadedPhoto(input_file))
        .caption(&format!("[CACHE] {}/{}", subfolder, name))
        .await
        .context("Ошибка отправки медиа в канал")?;

    // 5. Извлекаем метаданные фото
    let photo = msg
        .media()
        .and_then(|m| match m {
            Media::Photo(p) => Some(p),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Не удалось извлечь фото из сообщения"))?;

    // 6. Сохраняем в БД
    let cached = CachedImage {
        name: name.to_string(),
        subfolder: subfolder.to_string(),
        file_id: photo.id(),
        access_hash: photo.access_hash(),
        file_reference: photo.file_reference().to_vec(),
    };
    save_to_db(db, &cached).await?;

    Ok(cached.to_input_photo())
}

/// Получает изображение из БД по имени
async fn get_from_db(db: &PgClient, name: &str) -> Result<Option<CachedImage>> {
    let row = db
        .query_opt(
            r#"
            SELECT name, subfolder, file_id, access_hash, file_reference
            FROM ob_image_cache
            WHERE name = $1
            "#,
            &[&name],
        )
        .await?
        .map(|r| {
            Ok(CachedImage {
                name: r.try_get("name")?,
                subfolder: r.try_get("subfolder")?,
                file_id: r.try_get("file_id")?,
                access_hash: r.try_get("access_hash")?,
                file_reference: r.try_get("file_reference")?,
            })
        });

    match row {
        Some(Ok(cached)) => Ok(Some(cached)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

/// Сохраняет изображение в БД
async fn save_to_db(db: &PgClient, cached: &CachedImage) -> Result<()> {
    db.execute(
        r#"
        INSERT INTO ob_image_cache (name, subfolder, file_id, access_hash, file_reference)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (name) DO UPDATE
        SET file_id = EXCLUDED.file_id,
            access_hash = EXCLUDED.access_hash,
            file_reference = EXCLUDED.file_reference,
            created_at = NOW()
        "#,
        &[
            &cached.name,
            &cached.subfolder,
            &cached.file_id,
            &cached.access_hash,
            &&cached.file_reference[..],
        ],
    )
    .await?;

    Ok(())
}
