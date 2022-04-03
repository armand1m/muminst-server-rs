use std::{
    fs::File,
    io::{Cursor, Error, ErrorKind, Write},
    path::Path,
};

use actix_web::web::{self, Bytes};
use sha256::digest_bytes;

pub async fn save_sound_as_file(
    memory_file: Cursor<Vec<u8>>,
    file_name: String,
    extension: String,
    audio_folder_path: &Path,
) -> Result<File, Error> {
    let mut filepath = audio_folder_path.join(&file_name);
    filepath.set_extension(extension);

    /*
     * Load buffer from memory file into the
     * actual filesystem file
     */
    let file = web::block(move || {
        let mut file = File::create(filepath).expect("File to be created");
        file.write_all(memory_file.get_ref())
            .expect("File content to be written to filesystem");
        file
    })
    .await
    .expect("File content to be written to filesystem");

    Ok(file)
}

pub async fn validate_sound(
    file_content: Vec<Bytes>,
) -> Result<(Cursor<Vec<u8>>, infer::Type, String), Error> {
    /*
     * Creates a Cursor to load the buffer
     * in memory before actually writing it
     * to the disk
     */
    let mut memory_file = Cursor::new(Vec::<u8>::new());
    let mut content_iter = file_content.iter().cloned();

    for chunk in content_iter.by_ref() {
        memory_file = web::block(move || memory_file.write_all(&chunk).map(|_| memory_file))
            .await
            .expect("File content to be written")?;
    }

    let memory_file_buf = memory_file.get_ref();
    let file_type_slice = &memory_file_buf[0..4];
    let file_type = match infer::get(file_type_slice) {
        Some(file_type) => file_type,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Failed to identify the file type.",
            ));
        }
    };

    let file_extension = file_type.extension();
    let valid_extensions = ["mp3", "wav", "ogg", "webm"];

    if !valid_extensions.contains(&file_extension) {
        return Err(Error::new(ErrorKind::InvalidData, "File type is not valid"));
    }

    /*
     * Create a SHA256 hash from the buffer
     */
    let file_hash = digest_bytes(memory_file_buf);

    Ok((memory_file, file_type, file_hash))
}
