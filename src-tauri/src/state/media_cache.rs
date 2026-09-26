use super::*;

impl AppCore {
    pub async fn cache_artwork(&self, id: String, artwork: EmbeddedArtwork) {
        let mut cache = self.media_artwork.write().await;
        cache.retain(|item| item.id != id);
        cache.push_front(MediaArtwork {
            id,
            content_type: artwork.content_type,
            bytes: Bytes::from(artwork.bytes),
        });
        cache.truncate(ARTWORK_CACHE_LIMIT);
    }

    pub(super) async fn cached_artwork_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.media_artwork
            .read()
            .await
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.bytes.to_vec())
    }

    pub async fn cache_audio(&self, id: String, content_type: String, bytes: Vec<u8>) {
        let mut cache = self.media_audio.write().await;
        cache.retain(|item| item.id != id);
        cache.push_front(MediaAudio {
            id,
            content_type,
            bytes: Bytes::from(bytes),
        });
        while cache.len() > MEDIA_AUDIO_CACHE_LIMIT
            || cache.iter().map(|item| item.bytes.len()).sum::<usize>()
                > MEDIA_AUDIO_CACHE_BYTE_LIMIT
        {
            cache.pop_back();
        }
    }

    pub async fn claim_embed(&self, id: String) -> bool {
        let mut processed = self.processed_embed_ids.write().await;
        if processed.contains(&id) {
            return false;
        }
        processed.push_front(id);
        processed.truncate(PROCESSED_EMBED_LIMIT);
        true
    }

    pub async fn cache_media(&self, id: String, content_type: String, bytes: Vec<u8>) {
        let mut cache = self.cached_media.write().await;
        cache.retain(|item| item.id != id);
        cache.push_front(CachedMedia {
            id,
            content_type,
            bytes: Bytes::from(bytes),
        });
        while cache.len() > MEDIA_CACHE_ITEM_LIMIT
            || cache.iter().map(|item| item.bytes.len()).sum::<usize>() > MEDIA_CACHE_BYTE_LIMIT
        {
            cache.pop_back();
        }
    }

    pub async fn cached_media_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.cached_media
            .read()
            .await
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.bytes.to_vec())
    }
}
