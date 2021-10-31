use std::path::{Path, PathBuf};

// TODO: this is a dummy implementation of this function.
// hook this function into the storage implementation
// once ready to fetch the actual file name for the sound id
pub fn get_audio_path(audio_folder_path: &Path, _sound_id: String) -> PathBuf {
    let filename = "ecbcecb6-e82b-4aeb-8716-8f39b0446d36.mp3";
    audio_folder_path.join(filename)
}
