use std::{
    fs,
    io,
    path::PathBuf,
};
use tinytemplate::TinyTemplate;
use regex::Regex;

use crate::{Course, CourseContext, Config};

fn init_lecture(course: &Course, config: &Config) {
    let template_stream = fs::read(&config.template)
        .expect("Couldn't open template.");
    let template_string = String::from_utf8_lossy(&template_stream);

    let mut template = TinyTemplate::new();
    template.add_template("main", &template_string)
        .expect("Failed initializing template.");

    let context = CourseContext::from(&course, &config);

    let rendered = template.render("main", &context)
        .expect("Failed applying template");

    let lecture_directory = config.root.join(&course.semester).join(&course.name)
        .join("lecture");
    fs::write(lecture_directory.join("main.tex"), rendered)
        .expect("Failed making main.tex for course.");
}

fn detect_lessons(path: &PathBuf) -> io::Result<Vec<u32>> {
    let mut lesson_numbers: Vec<u32> = Vec::new();
    let dir_iter = fs::read_dir(path)?;
    let pattern = Regex::new(r"les(\d+)\.tex").unwrap();

    for entry in dir_iter {
        let entry_path = entry?.path();
        let name = entry_path.file_name().unwrap().to_string_lossy();
        if let Some(caps) = pattern.captures(&name) {
            let lesson_number = caps[1].parse::<u32>().unwrap();
            lesson_numbers.push(lesson_number);
        }
    }

    lesson_numbers.sort();

    return Ok(lesson_numbers);
}

fn update_main(main_path: &PathBuf, new_lessons: String) -> io::Result<()> {
    let main = fs::read(main_path)?;
    let pattern = Regex::new(r"% start lessons\n( {4}\\input\{les(\d+)\.tex\}\n)* {4}% end lessons")
        .unwrap();
    let fmt_lessons = format!("% start lessons\n{}    % end lessons", new_lessons);
    let main_string = String::from_utf8_lossy(&main);
    let new_main = pattern.replace(&main_string, fmt_lessons);
    fs::write(main_path, new_main.as_bytes())?;

    Ok(())
}

pub fn new_lesson(course: &Course, config: &Config) -> String {
    let lecture_directory = config.root.join(&course.semester).join(&course.name)
        .join("lecture");

    if !lecture_directory.try_exists().unwrap() {
        fs::create_dir_all(&lecture_directory).unwrap();
        init_lecture(course, config);
    }

    let mut lessons = detect_lessons(&lecture_directory)
        .expect("Could not make sense of the lesson situation.");
    let new_lesson = lessons.last().unwrap_or(&0).clone() + 1;

    let lesson_file = format!("les{new_lesson}.tex");
    let lesson_path = lecture_directory.join(lesson_file.clone());
    fs::write(&lesson_path, format!("\\lesson{{{new_lesson}}}{{}}\n\n").as_bytes())
        .expect("Unable to write to lesson file.");

    lessons.push(new_lesson);

    let mut lessons_string = String::new();

    for num in lessons {
        lessons_string.push_str(format!("    \\input{{les{num}.tex}}\n").as_str());
    }

    update_main(&lecture_directory.join("main.tex"), lessons_string)
        .expect("Unable to update main.tex");
    return lesson_file;
}
