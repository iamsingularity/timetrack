use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use TimeTracker;

mod raw_log;
use self::raw_log::{raw_logs_from};

mod span;
use self::span::{Span, get_spans_from};

mod display;
use self::display::display;
use self::span::get_last_timestamp_per_project;
use calc::raw_log::RawLog;
use calc::span::spans_from;

impl<'a> TimeTracker<'a> {

    pub fn calc(&self) {
        // process raw data into spans
        let mut raw_data = String::new();
        {
            let mut raw_data_file = OpenOptions::new()
                .read(true)
                .open(&self.config.raw_data_path).unwrap();
            raw_data_file.read_to_string(&mut raw_data)
                .expect("something went wrong reading the file");
        }
        let new_spans = get_spans_from(raw_logs_from(raw_data));

        // append spans to processed data file
        {
            let mut processed_data_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open(&self.config.processed_data_path).unwrap();

            for span in &new_spans {
                writeln!(&mut processed_data_file, "{}", span);
            }
        }

        // overwrite raw data file with last timestamp for each project (note this could cause small amount of data loss)
        let last_timestamp_per_project = get_last_timestamp_per_project(&new_spans);
        let mut updated_raw_log = String::new();
        {
            use std::fmt::Write;
            for (project_name, timestamp) in last_timestamp_per_project.into_iter() {
                let raw_log = RawLog {
                    name: project_name,
                    timestamp,
                };
                writeln!(&mut updated_raw_log, "{}", raw_log);
            }
        }
        {
            let mut raw_data_file = OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(true)
                .open(&self.config.raw_data_path).unwrap();
            write!(&mut raw_data_file, "{}", updated_raw_log);
        }

        // process spans from processed file as normal
        let mut processed_data_file = OpenOptions::new()
            .read(true)
            .open(&self.config.processed_data_path).unwrap();
        let mut all_spans_string = String::new();
        processed_data_file.read_to_string(&mut all_spans_string)
            .expect("Failed to read processed data");
        let all_spans = spans_from(all_spans_string);

        display(&calculate_project_total_time(all_spans));
    }

}

fn calculate_project_total_time(spans: Vec<Span>) -> HashMap<String, u64> {
    let mut project_totals = HashMap::new();

    for span in spans {
        let span_duration = span.duration();
        let span_name = span.name;

        if project_totals.contains_key(&span_name) {
            let old_total = project_totals.remove(&span_name).unwrap();
            project_totals.insert(span_name, old_total + span_duration);
        } else {
            project_totals.insert(span_name, span_duration);
        };
    }

    project_totals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_project_total_time_single_project() {
        let mut spans = vec![];
        let proj_1_name = "proj1";
        spans.push(Span { name: String::from("proj1"), start:1, end: 5});
        spans.push(Span { name: String::from("proj1"), start: 11, end: 26});

        let project_totals = calculate_project_total_time(spans);

        assert!(project_totals.contains_key(proj_1_name));
        assert_eq!(19u64, *project_totals.get(proj_1_name).unwrap());
    }

    #[test]
    fn calculate_project_total_time_two_projects() {
        let mut spans = vec![];
        let proj_1_name = "proj1";
        let proj_2_name = "proj2";
        spans.push(Span { name: String::from("proj1"), start: 1, end: 5});
        spans.push(Span { name: String::from("proj2"), start: 7, end: 12});
        spans.push(Span { name: String::from("proj1"), start: 11, end: 26});

        let project_totals = calculate_project_total_time(spans);

        assert!(project_totals.contains_key(proj_1_name));
        assert_eq!(19u64, *project_totals.get(proj_1_name).unwrap());
        assert!(project_totals.contains_key(proj_2_name));
        assert_eq!(5u64, *project_totals.get(proj_2_name).unwrap());
    }
}
