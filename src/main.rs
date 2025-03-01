mod mkvxml;
use log::{info, warn, error};
use serde::Deserialize;
use serde_xml_rs::from_str;
use clap::{value_parser, Arg, Command};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::process::Command as stdCommand;
use std::process::Stdio;
use std::path::{Path, PathBuf};
use std::time::Instant;
use image::{open, GenericImageView};
use std::thread;
use indicatif::{ProgressBar,ProgressStyle};
use walkdir::WalkDir;
use env_logger::{Builder, Target};

fn main() -> io::Result<()> {
    if let Err(e) = setup_logger() {
        eprintln!("Failed to setup logger: {}", e);
    }
    info!("Starting NFO2tags application");

    let matches = Command::new("NFO2tags")
        .version("1.0.2")
        .author("William Moore <bmoore@tekgnosis.works>")
        .about("Adds NFO information to the metadata in MP4 or MKV files.")
        .arg(
            Arg::new("video")
                .short('v')
                .long("video")
                .value_name("File.mp4")
                .value_parser(value_parser!(PathBuf))
                .help("Sets the input video file. Use parent folder for multiple files.")
                .required(true),
        )
        .arg(
            Arg::new("nfo")
                .short('n')
                .long("nfo")
                .value_name("File.nfo")
                .value_parser(value_parser!(PathBuf))
                .help("Sets the input NFO file"),
        )
        .arg(
            Arg::new("cover")
                .short('c')
                .long("cover")
                .value_name("Cover.jpg")
                .value_parser(value_parser!(PathBuf))
                .help("Sets the cover file, either jpg or png. If using folder, this does not work. It uses the video file name + texted passed in to -N or --cover-name. Default is '-poster'"),
        )
        .arg(
            Arg::new("cover-name")
                .short('N')
                .long("cover-name")
                .value_name("File-poster.jpg")
                .help("Custom suffix for cover images")
                .default_value("-poster"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FinishedFile.mp4")
                .value_parser(value_parser!(PathBuf))
                .help("Sets mp4's output file, since the whole container must be rewritten to put in the tags. If missing, it just creats a backup of the file, File.OLD.mp4. ***Does not apply to MVK***"),
        )
        .arg(
            Arg::new("delete")
                .short('d')
                .long("delete")
                .action(clap::ArgAction::SetTrue)
                .help("Delete the OLD files after processing"),
        )
        .get_matches();

    let ffmpegtest: bool = check_for_programs("ffmpeg");
    let mkvpropedittest: bool = check_for_programs("mkvpropedit");
    if !ffmpegtest || !mkvpropedittest {
        if !ffmpegtest {
            error!("ffmpeg is not installed. Visit https://www.ffmpeg.org/ or use your package manager to install.");
        }
        if !mkvpropedittest {
            error!("mkvpropedit (part of mkvtoolnix) is not installed. Visit https://mkvtoolnix.download/ or use your package manager to install.");
        }
        return Ok(());
    }

    let video_path: &PathBuf = matches.get_one("video")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound,"Video File not specified"))?;
    let cover_suffix: &String = matches.get_one("cover-name").unwrap();
    let deletefile = matches.get_flag("delete");
    let start_time = Instant::now();
    let mut processed_count = 0;
    let mut error_count = 0;

    if video_path.is_dir() {
        info!("Processing directory: {}", video_path.display());
        for entry in WalkDir::new(video_path).into_iter().filter_map(|e|e.ok()){
            let path = entry.path();
            if let Some(ext)= path.extension().and_then(|e|e.to_str()){
                if ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("mkv") {
                    let passnfo = nfo_path(path.to_path_buf(),None);
                    let passcover = cover_path(path.to_path_buf(),None,cover_suffix.to_string());
                    let passoutput = output_file_path(path.to_path_buf(),None);
                    match process_file(path, passnfo.as_ref().map(|p|p.as_path()), passcover.as_ref().map(|p|p.as_path()),passoutput.as_ref().map(|p|p.as_path()),deletefile ) {
                        Ok(_) => {
                            processed_count += 1;
                            info!("Success: {}", path.display());
                        }
                        Err(e) => {
                            error_count += 1;
                            warn!("Error Processing: \n {} \n {}",path.display(),e);
                        }
                    }
                }
            }
        }
    } else {
        info!("Processing single file: {}", video_path.display());
        let coversuffix: &String = matches.get_one("cover-name").unwrap();
        let passnfo = nfo_path(video_path.to_path_buf(),matches.get_one("nfo"));
        let passcover = cover_path(video_path.to_path_buf(),matches.get_one("cover"),coversuffix.to_string());
        let passoutput = output_file_path(video_path.to_path_buf(),matches.get_one("output"));
        match process_file(video_path, passnfo.as_ref().map(|p|p.as_path()), passcover.as_ref().map(|p|p.as_path()),passoutput.as_ref().map(|p|p.as_path()),deletefile ) {
            Ok(_) => {
                info!("Success: {}", passoutput.unwrap().display());
                processed_count += 1;
            }
            Err(e) => {
                warn!("Error Processing: \n {} \n {}",video_path.display(),e);
            }
        }
    }

    let duration = start_time.elapsed();
    info!("Processing completed in {:?}", duration);
    info!("Files processed: {}", processed_count);
    if error_count > 0 {
        warn!("Files with errors: {}", error_count);
    }

    Ok(())
}

fn process_file(
    video_path: &Path,
    nfo_path: Option<&Path>,
    cover_path: Option<&Path>,
    output_path: Option<&Path>,
    deletefile: bool
) -> io::Result<()> {
    let mut use_nfo = true;
    let mut nfo: Option<Nfo> = None;
    let mut genres = String::new();
    let mut tags = String::new();

    if let Some(nfo_file_path) = nfo_path {
        if !nfo_file_path.exists() {
            println!("No NFO file found at provided address: {}", nfo_file_path.display());
            use_nfo = false;
        } else {
            let mut nfo_file = File::open(nfo_file_path)?;
            let mut nfo_content = String::new();
            nfo_file.read_to_string(&mut nfo_content)?;
            let nfo_data: Nfo = from_str(&nfo_content).map_err(|e| {
                Error::new(ErrorKind::InvalidData, format!("Failed to parse NFO: {}", e))
            })?;
            genres = nfo_data.genre.iter().map(|g| g.value.as_str()).collect::<Vec<_>>().join(",");
            tags = nfo_data.tags.iter().map(|t| t.value.as_str()).collect::<Vec<_>>().join(",");
            nfo = Some(nfo_data);
        }
    } else {
        use_nfo = false;
    }

    let mut cover_type = "image/jpeg";
    let mut use_cover = false;
    match cover_path {
        Some(_) => {
            if cover_path.unwrap().extension().and_then(|ext| ext.to_str()) == Some("png") {
                cover_type = "image/png";
            }
            use_cover = true;
        }
        None => {
            use_cover = false;
        }
    }

    let is_landscape = if let Some(cover_file_path) = cover_path {
        let landscape = open(cover_file_path).map_err(|e| {
            Error::new(ErrorKind::InvalidData, format!("Failed to open image: {}", e))
        })?;
        let (width, height) = landscape.dimensions();
        width > height
    } else {
        false
    };

    let mut output_xml_path = PathBuf::from(video_path);
    output_xml_path.set_extension("xml");

    let file_ext = video_path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid file extension"))?;

    if !use_nfo && !use_cover {
        warn!("Not Processing: Due to no NFO nor cover file: {}",video_path.display());
        return Err(Error::new(
            ErrorKind::NotFound,
            "NFO and Cover are missing or invalid."
        ));
    }

    let nfo_data = nfo.ok_or_else(|| Error::new(ErrorKind::NotFound, "NFO data not available"))?;
    
    let title_metadata = format!("title={}", nfo_data.title);
    let genre_metadata = format!("genre={}", genres);
    let keywords_metadata = format!("keywords={}", tags);
    let description_metadata = format!("description={}", nfo_data.plot);
    let synopsis_metadata = format!("synopsis={}", nfo_data.outline);
    let date_metadata = format!("date={}", nfo_data.premiered);
    let mkv_xml_file = format!("all:{}", output_xml_path.to_str().unwrap_or(""));
    let mkv_cover = if is_landscape { "cover_land" } else { "cover" };

    match file_ext {
        "mp4" => {
            let mut working_video_path = PathBuf::new();
            if video_path == output_path.unwrap() {
                let mut filename: OsString = OsString::from(video_path.file_stem().unwrap());
                filename.push(".OLD.");
                filename.push(video_path.extension().unwrap().to_str().unwrap());
                let mut newpath: PathBuf = video_path.to_path_buf();
                newpath.set_file_name(filename);
                working_video_path = newpath;
            }
            fs::rename(&video_path, &working_video_path)?;
            let mut ffmpeg_args = vec!["-nostats", "-loglevel", "0", "-i", working_video_path.to_str().unwrap_or("")];
            
            if use_cover {
                if let Some(cover) = cover_path {
                    ffmpeg_args.extend_from_slice(&["-i", cover.to_str().unwrap_or(""), "-map", "1", "-map", "0"]);
                }
            }

            ffmpeg_args.extend_from_slice(&[
                "-metadata", &title_metadata,
                "-metadata", &genre_metadata,
                "-metadata", &keywords_metadata,
                "-metadata", &description_metadata,
                "-metadata", &synopsis_metadata,
                "-metadata", &date_metadata,
                "-codec", "copy",
            ]);

            if use_cover {
                ffmpeg_args.extend_from_slice(&["-disposition:0", "attached_pic"]);
            }

            if let Some(output) = output_path {
                ffmpeg_args.push(output.to_str().unwrap_or(""));
            }

            let didcomplete = run_ffmpeg_with_progress(&working_video_path.to_str().unwrap(), ffmpeg_args);
            match didcomplete {
                Ok(_) => {
                    if deletefile {
                        let _ = fs::remove_file(&working_video_path);
                    }
                },
                Err(e) => { error!("FFMpeg did not complete: {}", e)}
            }
        },
        "mkv" => {
            if let Some(nfo_path) = nfo_path {
                let _ = mkvxml::convert_to_mkv_tags(
                    nfo_path.to_str().unwrap_or(""),
                    output_xml_path.to_str().unwrap_or("")
                );
            }

            let mut mkvpropedit_args = vec![
                "--edit", "info",
                "-s", &title_metadata,
                video_path.to_str().unwrap_or(""),
                "--tags", &mkv_xml_file
            ];

            if let Some(cover) = cover_path {
                if cover.exists() {
                    mkvpropedit_args.extend_from_slice(&[
                        "--attachment-name", mkv_cover,
                        "--attachment-mime-type", cover_type,
                        "--add-attachment", cover.to_str().unwrap_or("")
                    ]);
                }
            }

            stdCommand::new("mkvpropedit")
                .args(&[video_path.to_str().unwrap_or(""), "--delete-attachment", "mime-type:image/jpeg"]);
            stdCommand::new("mkvpropedit")
                .args(&[video_path.to_str().unwrap_or(""), "--delete-attachment", "mime-type:image/png"]);
            stdCommand::new("mkvpropedit")
                .args(&[video_path.to_str().unwrap_or(""), "--tags", "all:"]);
            let runthis = stdCommand::new("mkvpropedit").args(&mkvpropedit_args).output();
            match runthis {
                Ok(_) => {
                },
                Err(e) => {
                    error!("mkvpropedit error: {}",e)
                }
            }
            let _ = fs::remove_file(output_xml_path);
        },
        _ => error!("{} Incorrect file type. It only works with MP4 and MKV files.", file_ext),
    }
    info!("File is complete: {}", video_path.display());
    Ok(())
}

fn run_ffmpeg_with_progress(input_file: &str, mut video_args: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Keep track of whether we're getting progress updates
    let mut received_progress = false;
    
    // Get the duration before starting ffmpeg
    let duration = {
        let probe = stdCommand::new("ffprobe")
            .args([
                "-v", "quiet",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                input_file
            ])
            .output()?;
        
        String::from_utf8_lossy(&probe.stdout)
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
    };

    // Add progress output flag to FFmpeg
    let input_index = video_args.iter().position(|&x| x == input_file).unwrap_or(0);
    video_args.insert(input_index + 1, "-progress");
    video_args.insert(input_index + 2, "pipe:1");
    
    println!("Starting to process: {}", input_file);

    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})")?
        .progress_chars("#>-"));
    
    pb.set_position(0);
    
    let mut cmd = stdCommand::new("ffmpeg")
        .args(&video_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())  
        .spawn()?;
    
    let pb_clone = pb.clone();
    let fake_updates = thread::spawn(move || {
        for i in 1..100 {
            thread::sleep(std::time::Duration::from_millis(100));
            if !received_progress {
                pb_clone.set_position(i);
            } else {
                break;
            }
        }
    });
    
    if let Some(stdout) = cmd.stdout.take() {
        let reader = BufReader::new(stdout);
        
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.starts_with("out_time=") {
                    received_progress = true;  // We've received real progress
                    
                    let time_str = &line[9..];
                    if let Ok(seconds) = parse_timestamp(time_str) {
                        let percent = if duration > 0.0 {
                            ((seconds / duration) * 100.0) as u64
                        } else {
                            0
                        };
                        
                        pb.set_position(percent.min(100));
                    }
                }
            }
        }
    }
    
    // Wait for FFmpeg to complete
    let status = cmd.wait()?;
    
    // Cancel our fake updates thread
    fake_updates.join().unwrap_or(());
    
    // Always show 100% when done if successful
    if status.success() {
        pb.set_position(100);
        pb.finish_with_message("Done!");
    } else {
        pb.finish_with_message("Failed!");
        return Err("FFmpeg command failed".into());
    }
    
    Ok(())
}

fn parse_timestamp(timestamp: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = timestamp.split(':').collect();
    if parts.len() != 3 {
        return Err("Invalid timestamp format".into());
    }
    
    let hours: f64 = parts[0].parse()?;
    let minutes: f64 = parts[1].parse()?;
    let seconds: f64 = parts[2].parse()?;
    
    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

fn cover_path(mut path: PathBuf, cover_path: Option<&PathBuf>,suffix: String) -> Option<PathBuf>{
    match cover_path {
        Some(_)=> {
            let new_path_name: PathBuf = cover_path.unwrap().to_path_buf();
            if !is_correct_image(&new_path_name.extension().unwrap().to_str().unwrap()){
                warn!("Incorrect type. Must be PNG, JPG, or JPEG");
                return None
            }
            if !new_path_name.exists() {
                warn!("Cover file does not exist: {}",new_path_name.display());
                return None;
            }
            info!("Found cover file: {}",new_path_name.display());
            return Some(new_path_name)
        }
        None => {
            let cover_suffix = OsString::from(suffix);
            let mut cover_name = OsString::from(path.file_stem().unwrap());
            cover_name.push(cover_suffix);
            cover_name.push(".jpg");
            path.set_file_name(cover_name);
            if path.exists() {
                info!("Found cover file: {}", path.display());
                return Some(path)
            }
            path.set_extension("jpeg");
            if path.exists() {
                info!("Found cover file: {}", path.display());
                return Some(path)
            }
            path.set_extension("png");
            if path.exists() {
                info!("Found cover file: {}", path.display());
                return Some(path)
            } else{
                warn!("A cover file was not found.");
                return None
            }
        }
    }
}

fn is_correct_image(image: &str) -> bool{
    matches!( image, "jpg" | "jpeg" | "png")
}

fn output_file_path(path: PathBuf, output_file_path: Option<&PathBuf>) -> Option<PathBuf>{
    match output_file_path {
        Some(_) => {
            return Some(output_file_path.unwrap().to_path_buf());
        }
        None => {
            return Some(path)
        }
    }
}

fn nfo_path(mut path: PathBuf, nfo_cli_option: Option<&PathBuf>) -> Option<PathBuf>{
    match nfo_cli_option {
        Some(_) => {
            let nfo_check = nfo_cli_option.unwrap().to_path_buf();
            if nfo_check.exists() {
                info!("Found NFO file: {}", nfo_check.display());
                return Some(nfo_check)
            }
            warn!("NFO file not found at {}", nfo_check.display());
            return None
        }
        None => {
            path.set_extension("nfo");
            if path.exists() {
                info!("Found NFO file: {}", path.display());
                return Some(path);
            } else {
                warn!("No NFO file found for video: {}", path.display());
                return None;
            }
        }
    }
}

fn setup_logger() -> Result<(), io::Error> {
    let current_exe_dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not get executable directory"))?
        .to_path_buf();
    
    let log_file_path = current_exe_dir.join("nfo2tags.log");
    
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    let multi_writer = MultiWriter::new(vec![
        Box::new(file),
        Box::new(io::stderr()),
    ]);

    let mut builder = Builder::from_default_env();

    builder.filter_level(log::LevelFilter::Info);
    
    builder
        .target(Target::Pipe(Box::new(multi_writer)))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    Ok(())
}

fn check_for_programs(program_name: &str) -> bool{

    match stdCommand::new(program_name).spawn() {
        Ok(mut process) => {
            process.kill().unwrap_or_default();
            return true;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (), 
        Err(_) => return false, 
    }

    let common_paths = if cfg!(windows) {
        vec![
            format!("C:\\Program Files\\FFmpeg\\bin\\{}.exe", program_name),
            format!("C:\\Program Files (x86)\\FFmpeg\\bin\\{}.exe", program_name),
            format!("C:\\Program Files\\MKVToolnix\\{}.exe", program_name), 
            format!("C:\\Program Files (x86)\\MKVToolnix\\{}.exe", program_name),
            format!("C:\\ffmpeg\\bin\\{}.exe", program_name),
            format!("C:\\mkvtoolnix\\{}.exe", program_name),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            format!("/usr/local/bin/{}", program_name),
            format!("/opt/homebrew/bin/{}", program_name),
            format!("/Applications/MKVToolnix.app/Contents/MacOS/{}", program_name),
        ]
    } else { 
        vec![
            format!("/usr/bin/{}", program_name),
            format!("/usr/local/bin/{}", program_name),
            format!("/bin/{}", program_name),
            format!("/opt/bin/{}", program_name),
        ]
    };

    for path in common_paths {
        if Path::new(&path).exists() {
            return true;
        }
    }

    false
}

#[derive(Debug, Deserialize)]
struct Nfo {
    title: String,
    #[serde(default)]
    premiered: String,
    #[serde(default)]
    outline: String,
    #[serde(default)]
    plot: String,
    #[serde(default)]
    genre: Vec<Genre>,
    #[serde(rename = "tag", default)]
    tags: Vec<Tag>
}

#[derive(Debug, Deserialize)]
struct Genre{
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Debug, Deserialize)]
struct Tag{
    #[serde(rename = "$value")]
    value: String,
}

struct MultiWriter {
    writers: Vec<Box<dyn Write + Send + 'static>>,
}

impl MultiWriter {
    fn new(writers: Vec<Box<dyn Write + Send + 'static>>) -> Self {
        MultiWriter { writers }
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for writer in &mut self.writers {
            writer.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        for writer in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }
}