use std::{
    path::PathBuf,
    io::{self, Write},
    process::{Command, Stdio},
    collections::HashMap,
    fs,
};
use lecture::new_lesson;
use homework::{new_homework, recent_homework, view_homeworks};
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand, Args};

mod lecture;
mod homework;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file location
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a course
    Init(InitArgs),

    /// Open notes for a course
    Open(OpenArgs),
}

#[derive(Args)]
struct InitArgs {
    /// Course prefix and number
    name: String,

    /// Course title
    title: String,

    /// Professor's name
    prof: String,

    /// Semester id for grouping courses
    semester: String,
}

#[derive(Args)]
struct OpenArgs {
    /// Course prefix and number
    name: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Config {
    root: PathBuf,
    lecture_template: PathBuf,
    homework_template: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct Course {
    name: String,
    title: String,
    prof: String,
    semester: String,
}

#[derive(Serialize)]
struct CourseContext<'a> {
    name: &'a String,
    title: &'a String,
    prof: &'a String,
    semester: &'a String,
    notebook: String,
}

impl<'a> CourseContext<'a> {
    fn from(course: &'a Course, config: &'a Config) -> CourseContext<'a> {
        let notebook = config.root.to_str()
            .unwrap().to_owned();

        CourseContext {
            name: &course.name,
            title: &course.title,
            prof: &course.prof,
            semester: &course.semester,
            notebook,
        }
    }
}

fn resolve_home(path: &mut PathBuf) {
    if path.starts_with("~") {
        let temp = path.strip_prefix("~").unwrap();
        *path = dirs::home_dir().expect("Could not resolve home directory.")
            .join(temp);
    }
}

fn create_config(path: &PathBuf) -> Config {
    let mut root: PathBuf;
    let mut input = String::new();

    print!("Where should lecture notes go? ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read user input.");

    root = PathBuf::from(input.trim());
    resolve_home(&mut root);

    let lecture_template = path.join("lecture_template.tex");
    let homework_template = path.join("lecture_template.tex");
    let config = Config { root, lecture_template, homework_template };

    let toml = toml::to_string(&config)
        .expect("Unable to convert config struct to toml string.");
    fs::write(path.join("config.toml"), toml)
        .expect("Unable to initialize config.");

    return config;
}

fn read_config(path: &PathBuf) -> Config {
    let file_path = path.join("config.toml");

    if !file_path.try_exists().unwrap() {
        return create_config(&path);
    }
    
    let file = fs::read_to_string(file_path)
        .expect("Error reading config file.");
    return toml::from_str(&file)
        .expect("Error parsing config file.");
}

fn get_courses(path: &PathBuf) -> HashMap<String, Course> {
    let file_path = path.join("courses.toml");
    let file = fs::read_to_string(file_path)
        .unwrap_or(String::new());

    return toml::from_str(&file).unwrap();
}

fn save_courses(courses: &HashMap<String, Course>, path: &PathBuf) {
    let toml = toml::to_string(courses)
        .expect("Unable to convert courses to toml string.");
    fs::write(path.join("courses.toml"), toml)
        .expect("Unable save courses information.");
}

fn pick_course<'a>(courses: &'a HashMap<String, Course>) -> Result<&'a Course, &'static str> {
    let mut options = String::new();
    let courses_seq: Vec<&Course> = courses.values().collect();
    for course in &courses_seq {
        options.push_str(&course.name.to_uppercase());
        options.push_str(": ");
        options.push_str(&course.title);
        options.push_str("\n");
    }

    let course_idx = rofi_picker("Courses", options)?;

    return Ok(&courses_seq[course_idx as usize]);
}

fn rofi_picker(title: &'static str, input: String) -> Result<u32, &'static str> {
    let mut rofi = Command::new("rofi")
        .args(["-dmenu", "-i"])
        .args(["-p", title])
        .args(["-format", "i"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn().map_err(|_| "Rofi failed to launch.")?;

    rofi.stdin.as_mut().unwrap().write(input.as_bytes())
        .map_err(|_| "Could not send input to Rofi process.")?;

    let rofi_output = rofi.wait_with_output()
        .map_err(|_| "Waiting on Rofi failed.")?;
    let choice = String::from_utf8(rofi_output.stdout)
        .map_err(|_| "Couldn't make sense of Rofi output.")?;

    if choice.trim().is_empty() { return Err("Rofi was quit without a choice."); }

    let choice_idx = choice.trim().parse::<u32>().unwrap();

    return Ok(choice_idx);
}

fn launch_tex(directory: &PathBuf, file_name: &String) {
    let mut xoppdog = Command::new("xoppdog")
        .arg("sit")
        .arg(directory.join("figures").as_os_str())
        .stdout(io::stdout())
        .stderr(io::stderr())
        .spawn()
        .expect("Failed to start xoppdog.");

    let mut wezterm = Command::new("wezterm")
        .args(["start", "--always-new-process", "--cwd"])
        .arg(directory.as_os_str())
        .args(["nvim", file_name])
        .spawn()
        .expect("Failed to start neovim terminal.");

    wezterm.wait().expect("Where did the terminal go??");
    xoppdog.kill().expect("Couldn't kill xoppdog.");
}

fn launch_pdf(directory: &PathBuf, file_name: &String) {
    Command::new("zathura")
        .arg(directory.join(file_name).as_os_str())
        .spawn()
        .expect("Failed to start zathura.");
}

fn init_command(args: InitArgs, config: &Config) -> Course {
    let course = Course {
        name: args.name,
        title: args.title,
        prof: args.prof, 
        semester: args.semester
    };

    let lecture_directory = config.root.join(&course.semester).join(&course.name);
    fs::create_dir_all(&lecture_directory)
        .expect("Failed creating course dir.");

    return course;
}

fn open_command(args: OpenArgs, courses: &HashMap<String, Course>, config: &Config) -> Result<(), &'static str> {
    let course = match &args.name {
        Some(name) => courses.get(name).expect("Course not found."),
        None => pick_course(courses)?,
    };

    let in_lecture = rofi_picker("Type", String::from("Lecture\nHomework"))?;

    let _ = match in_lecture {
        0 => open_lecture(&course, &config),
        _ => open_homework(&course, &config),
    };

    Ok(())
}

fn open_lecture(course: &Course, config: &Config) -> Result<(), &'static str>{
    let action = rofi_picker("Action", String::from("New Lesson\nEdit Notes\nView"))?;

    let course_directory = config.root.join(&course.semester).join(&course.name);
    let (course_directory, file_name) = match action {
        0 => new_lesson(&course, &config),
        1 => (course_directory.join("lecture"), String::from("main.tex")),
        _ => (course_directory.join("lecture"), String::from("main.pdf")),
    };

    match action {
        2 => launch_pdf(&course_directory, &file_name),
        _ => launch_tex(&course_directory, &file_name),
    };

    Ok(())
}

fn open_homework(course: &Course, config: &Config) -> Result<(), &'static str>{
    let action = rofi_picker("Action", String::from("Edit Recent\nNew Homework\nView Previous"))?;

    let (course_directory, file_name) = match action {
        0 => recent_homework(&course, &config),
        1 => new_homework(&course, &config),
        _ => view_homeworks(&course, &config)?,
    };

    launch_tex(&course_directory, &file_name);

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let config_path = dirs::config_dir()
        .expect("Could not resolve config directory.")
        .join("lectern");

    if !config_path.try_exists().unwrap() {
        fs::create_dir_all(&config_path).unwrap();
    }

    let config = read_config(&config_path);
    let mut courses = get_courses(&config_path);

    match cli.command {
        Commands::Init(args) => {
            let course = init_command(args, &config);
            let name = course.name.clone();
            courses.insert(name, course);
            save_courses(&courses, &config_path);
        },
        Commands::Open(args) => open_command(args, &courses, &config).unwrap(),
    };
}
