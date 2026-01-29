use rustrtc::media::MediaError;

fn main() {
    // This will help us understand what variants are available
    let _ = MediaError::Other("test".to_string());
    let _ = MediaError::IOError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
}
