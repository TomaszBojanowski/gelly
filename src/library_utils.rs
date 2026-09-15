use crate::application::Application;
use crate::backend::BackendError;
use crate::models::{PlaylistModel, SongModel};

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<SongModel>, BackendError>) + 'static,
) {
    let library = app.library();
    let playlist_type = playlist_model.playlist_type();

    // Smart playlists work on the main thread - return immediately
    if playlist_type.is_smart() {
        cb(Ok(playlist_type.smart_songs(&library)));
        return;
    }

    // Regular playlists are fetched from the backend
    let id = playlist_type.to_id();
    let backend = app.backend();
    app.http_with_loading(
        async move { backend.get_playlist_items(&id).await },
        move |result| {
            cb(result.map(|items| {
                items
                    .items
                    .iter()
                    .map(|dto| SongModel::new(dto, library.song_is_favorite(&dto.id)))
                    .collect()
            }))
        },
    );
}

pub fn play_album(id: &str, app: &Application) {
    let songs = app.library().songs_for_album(id);
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0, false);
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_artist(id: &str, app: &Application) {
    let songs = app.library().songs_for_artist(id);
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0, false);
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_song(id: &str, app: &Application) {
    let songs = app.library().all_songs();
    let song = songs.iter().find(|s| s.id() == id);
    if let Some(audio_model) = app.audio_model() {
        if let Some(song) = song {
            audio_model.set_queue(vec![song.clone()], 0, true);
        }
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_instant_mix(id: &str, app: &Application) {
    let library = app.library();
    let seed = library.all_songs().into_iter().find(|s| s.id() == id);
    let backend = app.backend();
    let seed_id = id.to_string();
    let app_for_cb = app.clone();

    app.http_with_loading(
        async move { backend.get_similar_songs(&seed_id, 50).await },
        move |result| match result {
            Ok(items) => {
                let mut songs: Vec<SongModel> = seed.iter().cloned().collect();
                songs.extend(
                    items
                        .items
                        .iter()
                        .filter(|dto| seed.as_ref().is_none_or(|s| s.id() != dto.id))
                        .map(|dto| SongModel::new(dto, library.song_is_favorite(&dto.id))),
                );

                if let Some(audio_model) = app_for_cb.audio_model() {
                    audio_model.set_queue(songs, 0, true);
                } else {
                    log::warn!("No audio model found");
                }
            }
            Err(err) => log::warn!("Instant Mix failed: {err}"),
        },
    );
}
