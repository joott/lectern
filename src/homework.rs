use std::{
    fs,
    io,
    path::PathBuf,
};
use tinytemplate::TinyTemplate;
use regex::Regex;
use serde::Serialize;

use crate::{Course, Config, rofi_picker};

#[derive(Serialize)]
struct HomeworkContext<'a> {
    course: &'a String,
    number: u32,
}

fn display_name(name: &String) -> String {
    let pattern = Regex::new(r"([a-z]+)(\d{3})").unwrap();
    let caps = pattern.captures(name).unwrap();
    let dept = caps.get(1).unwrap().as_str().to_uppercase();
    let number = caps.get(2).unwrap().as_str();
    let display_name = format!("{dept} {number}");
    println!("{display_name}");

    return display_name;
}

fn init_homework(course: &Course, config: &Config, context: &HomeworkContext) -> (PathBuf, String) {
    let template_stream = fs::read(&config.homework_template)
        .expect("Couldn't open template.");
    let template_string = String::from_utf8_lossy(&template_stream);

    let mut template = TinyTemplate::new();
    template.add_template("main", &template_string)
        .expect("Failed initializing template.");

    let rendered = template.render("main", &context)
        .expect("Failed applying template");

    let homework_file = format!("homework{}.tex", context.number);
    let homework_directory = config.root.join(&course.semester).join(&course.name)
        .join(format!("homework{}", context.number));
    fs::create_dir(&homework_directory).unwrap();
    fs::write(homework_directory.join(homework_file.clone()), rendered)
        .expect("Failed making main tex for course.");

    return (homework_directory, homework_file);
}

fn detect_homeworks(path: &PathBuf) -> io::Result<Vec<u32>> {
    let mut homework_numbers: Vec<u32> = Vec::new();
    let dir_iter = fs::read_dir(path)?;
    let pattern = Regex::new(r"homework(\d+)").unwrap();

    for entry in dir_iter {
        let entry_path = entry?.path();
        let name = entry_path.file_name().unwrap().to_string_lossy();
        if let Some(caps) = pattern.captures(&name) {
            let homework_number = caps[1].parse::<u32>().unwrap();
            homework_numbers.push(homework_number);
        }
    }

    homework_numbers.sort();

    return Ok(homework_numbers);
}

pub fn new_homework(course: &Course, config: &Config) -> (PathBuf, String) {
    let course_directory = config.root.join(&course.semester).join(&course.name);

    if !course_directory.try_exists().unwrap() {
        fs::create_dir_all(&course_directory).unwrap();
    }

    let homeworks = detect_homeworks(&course_directory)
        .expect("Could not make sense of the homeworks situation.");
    let new_homework = homeworks.last().unwrap_or(&0).clone() + 1;

    let name = display_name(&course.name);
    let homework_context = HomeworkContext { course: &name, number: new_homework };
    return init_homework(course, config, &homework_context);
}

pub fn recent_homework(course: &Course, config: &Config) -> (PathBuf, String) {
    let course_directory = config.root.join(&course.semester).join(&course.name);

    let homeworks = detect_homeworks(&course_directory)
        .expect("Could not make sense of the homeworks situation.");
    let most_recent = homeworks.last().expect("No homeworks.");

    let homework_directory = course_directory.join(format!("homework{most_recent}"));
    let homework_file = format!("homework{most_recent}.tex");

    return (homework_directory, homework_file);
}

pub fn view_homeworks(course: &Course, config: &Config) -> Result<(PathBuf, String), &'static str> {
    let course_directory = config.root.join(&course.semester).join(&course.name);
    let homeworks = detect_homeworks(&course_directory)
        .expect("Could not make sense of the homeworks situation.");

    let mut homeworks_string = String::new();

    for (idx, hw) in homeworks.iter().enumerate() {
        if idx != 0 {
            homeworks_string.push_str("\n");
        }
        homeworks_string.push_str(&format!("Homework {hw}").to_string());
    }

    let hw_idx = rofi_picker("Homeworks", homeworks_string)?;

    let picked_homework = homeworks[hw_idx as usize];

    let homework_directory = course_directory.join(format!("homework{picked_homework}"));
    let homework_file = format!("homework{picked_homework}.tex");

    return Ok((homework_directory, homework_file));
}
