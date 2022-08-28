use std::sync::{Arc, Mutex};

use nconsole::{Command, CommandsRegister, Console, LogTypes};

use n_audio::player::Player;

struct PlayCommand {
    player: Arc<Mutex<Player>>,
}

impl PlayCommand {
    fn new(player: Arc<Mutex<Player>>) -> Self {
        PlayCommand { player }
    }
}

impl Command for PlayCommand {
    fn get_command_name(&self) -> &str {
        "play"
    }

    fn get_command_alias(&self) -> Vec<&str> {
        vec!["p"]
    }

    fn get_help(&self) -> &str {
        "Play a specific music file.\nUsage: play file.mp3"
    }

    fn on_command(&mut self, args: Vec<&str>) {
        if args.len() == 1 {
            let mut player = self.player.lock().unwrap();
            if !player.has_ended() {
                player.end_current().unwrap();
            }

            Console::log(LogTypes::INFO, String::from("Playing song"));
            player.play_from_path(args[0]).unwrap();
        }
    }
}

struct StopCommand {
    player: Arc<Mutex<Player>>,
}

impl StopCommand {
    fn new(player: Arc<Mutex<Player>>) -> Self {
        StopCommand { player }
    }
}

impl Command for StopCommand {
    fn get_command_name(&self) -> &str {
        "stop"
    }

    fn get_command_alias(&self) -> Vec<&str> {
        vec!["s"]
    }

    fn get_help(&self) -> &str {
        "Toggle pause the music.\nUsage: stop"
    }

    fn on_command(&mut self, _args: Vec<&str>) {
        let mut player = self.player.lock().unwrap();

        Console::log(LogTypes::INFO, String::from("Toggling pause"));

        if player.is_paused() {
            player.unpause().unwrap();
        } else {
            player.pause().unwrap();
        }
    }
}

struct VolumeCommand {
    player: Arc<Mutex<Player>>,
}

impl VolumeCommand {
    fn new(player: Arc<Mutex<Player>>) -> Self {
        VolumeCommand { player }
    }
}

impl Command for VolumeCommand {
    fn get_command_name(&self) -> &str {
        "volume"
    }

    fn get_command_alias(&self) -> Vec<&str> {
        vec!["v"]
    }

    fn get_help(&self) -> &str {
        "Sets the volume (min: 0.0, max: 1.0).\nUsage: volume 0.5"
    }

    fn on_command(&mut self, args: Vec<&str>) {
        if args.len() == 1 {
            let mut player = self.player.lock().unwrap();
            let mut volume = args[0].parse::<f32>().unwrap();
            volume = volume.min(1.0);
            volume = volume.max(0.0);

            Console::log(LogTypes::INFO, String::from("Setting volume"));

            player.set_volume(volume).unwrap();
        }
    }
}

struct SeekToCommand {
    player: Arc<Mutex<Player>>,
}

impl SeekToCommand {
    fn new(player: Arc<Mutex<Player>>) -> Self {
        SeekToCommand { player }
    }
}

impl Command for SeekToCommand {
    fn get_command_name(&self) -> &str {
        "seekto"
    }

    fn get_command_alias(&self) -> Vec<&str> {
        vec!["seek", "sk", "skt"]
    }

    fn get_help(&self) -> &str {
        "Seeks to the second specified.\nUsage: seek 10"
    }

    fn on_command(&mut self, args: Vec<&str>) {
        if args.len() == 1 {
            let player = self.player.lock().unwrap();
            let seekto = args[0].parse::<u64>().unwrap();

            Console::log(LogTypes::INFO, String::from("Seeking to"));

            player.seek_to(seekto, 0.0).unwrap();
        }
    }
}

struct PlaybackSpeedCommand {
    player: Arc<Mutex<Player>>,
}

impl PlaybackSpeedCommand {
    fn new(player: Arc<Mutex<Player>>) -> Self {
        PlaybackSpeedCommand { player }
    }
}

impl Command for PlaybackSpeedCommand {
    fn get_command_name(&self) -> &str {
        "speed"
    }

    fn get_command_alias(&self) -> Vec<&str> {
        vec!["sp"]
    }

    fn get_help(&self) -> &str {
        "Sets the playback speed.\nUsage: speed 2.0"
    }

    fn on_command(&mut self, args: Vec<&str>) {
        if args.len() == 1 {
            let mut player = self.player.lock().unwrap();
            let speed = args[0].parse::<f32>().unwrap();

            Console::log(LogTypes::INFO, String::from("Setting playback speed"));

            player.set_playback_speed(speed).unwrap();
        }
    }
}

fn main() {
    let player = Arc::new(Mutex::new(Player::new(
        String::from("N Cli Player"),
        1.0,
        1.0,
    )));
    let mut commands_register = CommandsRegister::new();

    commands_register.register_command(PlayCommand::new(player.clone()));
    commands_register.register_command(StopCommand::new(player.clone()));
    commands_register.register_command(SeekToCommand::new(player.clone()));
    commands_register.register_command(VolumeCommand::new(player.clone()));
    commands_register.register_command(PlaybackSpeedCommand::new(player.clone()));

    let mut console = Console::new(String::from(">>> "), commands_register);

    loop {
        console.update();
    }
}
