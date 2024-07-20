fn main() {
    // Install external dependency (in the shuttle container only)
    if std::env::var("SHUTTLE")
        .unwrap_or_default()
        .contains("true")
    {
        if !std::process::Command::new("apt")
            .arg("install")
            .arg("-y")
            .arg("pipx") // the apt package that a dependency of my project needs to compile
            .arg("ffmpeg") // the apt package that a dependency of my project needs to compile
            // can add more here
            .status()
            .expect("failed to run apt")
            .success()
        {
            panic!("failed to install dependencies")
        }
        if !std::process::Command::new("pipx")
            .arg("install")
            .arg("yt-dlp")
            // can add more here
            .status()
            .expect("failed to run pipx")
            .success()
        {
            panic!("failed to install dependencies")
        }
    }
}
