use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

use crate::models::{AcctDetail, AcctEntry, Job, JobState, Node, Partition};

pub struct SlurmParser;

impl SlurmParser {
    pub fn parse_squeue_output(output: &str) -> Result<Vec<Job>> {
        let mut jobs = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() || line.starts_with("JOBID") {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                let job_id = parts[0].trim().to_string();
                let name = parts[1].trim().to_string();
                let user = parts[2].trim().to_string();
                let state = JobState::from(parts[3].trim());

                let mut job = Job::new(job_id.clone(), name, user, state);

                // Parse array job ID if present (e.g., "23673084_5" -> array_job_id=23673084, task_id=5)
                if job_id.contains('_') {
                    let array_parts: Vec<&str> = job_id.split('_').collect();
                    if array_parts.len() == 2 {
                        job.array_job_id = Some(array_parts[0].to_string());
                        job.array_task_id = array_parts[1].parse().ok();
                    }
                }

                // Additional fields if present
                if parts.len() > 4 {
                    job.time_used = Some(parts[4].trim().to_string());
                }
                if parts.len() > 5 {
                    job.node_list = Some(parts[5].trim().to_string());
                }
                if parts.len() > 6 {
                    job.partition = parts[6].trim().to_string();
                }

                jobs.push(job);
            }
        }

        Ok(jobs)
    }

    pub fn parse_scontrol_output(output: &str) -> Result<HashMap<String, String>> {
        let mut fields = HashMap::new();

        // scontrol output format: "Key=Value Key2=Value2 ..." separated by whitespace.
        let re = Regex::new(r"(\w+)=(\S+)")?;

        for line in output.lines() {
            for cap in re.captures_iter(line) {
                let key = cap[1].to_string();
                let value = cap[2].trim_matches('"').to_string();
                fields.insert(key, value);
            }
        }

        Ok(fields)
    }

    pub fn enhance_job_with_scontrol_data(job: &mut Job, scontrol_fields: HashMap<String, String>) {
        if let Some(submit_time) = scontrol_fields.get("SubmitTime") {
            job.submit_time = Self::parse_slurm_time(submit_time);
        }

        if let Some(start_time) = scontrol_fields.get("StartTime") {
            job.start_time = Self::parse_slurm_time(start_time);
        }

        if let Some(end_time) = scontrol_fields.get("EndTime") {
            job.end_time = Self::parse_slurm_time(end_time);
        }

        if let Some(working_dir) = scontrol_fields.get("WorkDir") {
            job.working_dir = Some(working_dir.clone());
        }

        if let Some(std_out) = scontrol_fields.get("StdOut") {
            job.std_out = Some(std_out.clone());
        }

        if let Some(std_err) = scontrol_fields.get("StdErr") {
            job.std_err = Some(std_err.clone());
        }

        if let Some(nodes) = scontrol_fields.get("NumNodes") {
            job.nodes = nodes.parse().ok();
        }

        if let Some(cpus) = scontrol_fields.get("NumCPUs") {
            job.cpus = cpus.parse().ok();
        }

        if let Some(memory) = scontrol_fields.get("MinMemoryNode") {
            job.memory = Some(memory.clone());
        }

        if let Some(reason) = scontrol_fields.get("Reason") {
            job.reason = Some(reason.clone());
        }

        if let Some(exit_code) = scontrol_fields.get("ExitCode") {
            // Exit code format is usually "0:0" where first is exit code, second is signal
            if let Some(code) = exit_code.split(':').next() {
                job.exit_code = code.parse().ok();
            }
        }

        if let Some(time_limit) = scontrol_fields.get("TimeLimit") {
            job.time_limit = Some(time_limit.clone());
        }
    }

    fn parse_slurm_time(time_str: &str) -> Option<DateTime<Utc>> {
        // SLURM time formats: "2024-01-15T10:19:13" or "2024-01-15T10:19:13.123"
        // Sometimes also "Unknown" or "None" for jobs that haven't started
        if time_str == "Unknown" || time_str == "None" || time_str.is_empty() {
            return None;
        }

        // Try parsing with seconds
        if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S") {
            return Some(dt.and_utc());
        }

        // Try parsing with microseconds
        if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(dt.and_utc());
        }

        None
    }

    pub fn get_job_log_paths(job: &Job) -> Vec<String> {
        let mut paths = Vec::new();

        // Primary: Use the actual StdOut path from scontrol if available
        if let Some(std_out) = &job.std_out {
            paths.push(std_out.clone());
        }

        // Secondary: Use StdErr if different
        if let Some(std_err) = &job.std_err
            && Some(std_err) != job.std_out.as_ref()
        {
            paths.push(std_err.clone());
        }

        // Fallback: Common SLURM default patterns in working directory
        if let Some(work_dir) = &job.working_dir {
            paths.push(format!("{}/slurm-{}.out", work_dir, job.job_id));
            paths.push(format!("{}/slurm-{}.err", work_dir, job.job_id));
        } else {
            // If no working directory known, try current directory
            paths.push(format!("slurm-{}.out", job.job_id));
            paths.push(format!("slurm-{}.err", job.job_id));
        }

        // Additional fallback: Check /tmp for logs (common in dev environments)
        paths.push(format!("/tmp/slurm-{}.out", job.job_id));
        paths.push(format!("/tmp/slurm-{}.err", job.job_id));

        paths
    }

    /// Best-effort default log paths for a finished job. A job that used a
    /// custom `--output` path won't be found here.
    pub fn get_acct_log_paths(work_dir: &str, job_id: &str) -> Vec<String> {
        let mut paths = Vec::new();
        if !work_dir.is_empty() {
            paths.push(format!("{work_dir}/slurm-{job_id}.out"));
            paths.push(format!("{work_dir}/slurm-{job_id}.err"));
        }
        paths.push(format!("/tmp/slurm-{job_id}.out"));
        paths.push(format!("/tmp/slurm-{job_id}.err"));
        paths
    }

    /// Parse `sinfo_t_idle` output into partitions.
    /// Each line has the form: `Partition <name> : <N> nodes idle`
    pub fn parse_sinfo_t_idle(output: &str) -> Vec<Partition> {
        let mut partitions = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("Partition ") else {
                continue;
            };
            let Some(colon) = rest.find(':') else {
                continue;
            };
            let name = rest[..colon].trim().to_string();
            let after_colon = rest[colon + 1..].trim();
            let nodes_idle: u32 = after_colon
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);

            partitions.push(Partition {
                name,
                is_default: false,
                availability: "up".to_string(),
                nodes_alloc: 0,
                nodes_idle,
                nodes_other: 0,
                nodes_total: 0,
                time_limit: String::new(),
            });
        }

        partitions
    }

    /// Convert a `sinfo_t_idle` partition summary into synthetic `Node` entries
    /// (one per partition) so the Nodes tab can display idle counts.
    pub fn parse_sinfo_nodes(output: &str) -> Vec<Node> {
        Self::parse_sinfo_t_idle(output)
            .into_iter()
            .map(|p| Node {
                state: if p.nodes_idle > 0 {
                    "idle".to_string()
                } else {
                    "empty".to_string()
                },
                cpus_idle: p.nodes_idle,
                cpus_total: p.nodes_idle,
                partition: p.name.clone(),
                name: p.name,
                cpus_alloc: 0,
                cpus_other: 0,
                memory_mb: None,
                free_mem_mb: None,
                gres: None,
            })
            .collect()
    }

    /// Parse `sacct -X -n -P --format=JobID,JobName,State,ExitCode,Elapsed,Start,End`.
    pub fn parse_sacct(output: &str) -> Vec<AcctEntry> {
        let mut entries = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let f: Vec<&str> = line.split('|').collect();
            if f.len() < 7 {
                continue;
            }

            entries.push(AcctEntry {
                job_id: f[0].trim().to_string(),
                name: f[1].trim().to_string(),
                state: f[2].trim().to_string(),
                exit_code: f[3].trim().to_string(),
                elapsed: f[4].trim().to_string(),
                start: f[5].trim().to_string(),
                end: f[6].trim().to_string(),
            });
        }

        entries
    }

    /// Parse `sacct -j` detail. The largest MaxRSS across the step rows is
    /// folded in. `None` if the allocation row is missing.
    pub fn parse_sacct_detail(output: &str, job_id: &str) -> Option<AcctDetail> {
        let mut detail: Option<AcctDetail> = None;
        let mut max_rss_bytes = 0u64;
        let mut max_rss: Option<String> = None;

        for line in output.lines() {
            let f: Vec<&str> = line.split('|').collect();
            if f.len() < 17 {
                continue;
            }

            if f[0].trim() == job_id {
                detail = Some(AcctDetail {
                    job_id: f[0].trim().to_string(),
                    name: f[1].trim().to_string(),
                    user: f[2].trim().to_string(),
                    account: f[3].trim().to_string(),
                    partition: f[4].trim().to_string(),
                    node_list: f[5].trim().to_string(),
                    alloc_cpus: f[6].trim().to_string(),
                    req_mem: f[7].trim().to_string(),
                    max_rss: None,
                    total_cpu: f[9].trim().to_string(),
                    state: f[10].trim().to_string(),
                    exit_code: f[11].trim().to_string(),
                    submit: f[12].trim().to_string(),
                    start: f[13].trim().to_string(),
                    end: f[14].trim().to_string(),
                    elapsed: f[15].trim().to_string(),
                    work_dir: f[16].trim().to_string(),
                });
            }

            if let Some(bytes) = parse_mem_to_bytes(f[8].trim())
                && bytes >= max_rss_bytes
            {
                max_rss_bytes = bytes;
                max_rss = Some(f[8].trim().to_string());
            }
        }

        detail.map(|mut d| {
            d.max_rss = max_rss;
            d
        })
    }
}

/// Parse a sacct memory figure (`2072K`, `1.5G`) into bytes.
fn parse_mem_to_bytes(value: &str) -> Option<u64> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }

    let (num, scale) = match v.chars().last()? {
        'K' | 'k' => (&v[..v.len() - 1], 1024f64),
        'M' | 'm' => (&v[..v.len() - 1], 1024f64 * 1024.0),
        'G' | 'g' => (&v[..v.len() - 1], 1024f64 * 1024.0 * 1024.0),
        'T' | 't' => (&v[..v.len() - 1], 1024f64 * 1024.0 * 1024.0 * 1024.0),
        '0'..='9' => (v, 1.0),
        _ => return None,
    };

    num.parse::<f64>().ok().map(|n| (n * scale) as u64)
}

#[cfg(test)]
mod cluster_tests {
    use super::*;

    #[test]
    fn parses_sinfo_t_idle_partition_lines() {
        let raw = "Partition gpu                     :      5 nodes idle\n\
                   Partition cpu                     :      1 nodes idle\n\
                   Partition large                   :      0 nodes idle\n";
        let partitions = SlurmParser::parse_sinfo_t_idle(raw);
        assert_eq!(partitions.len(), 3);
        assert_eq!(partitions[0].name, "gpu");
        assert_eq!(partitions[0].nodes_idle, 5);
        assert_eq!(partitions[1].name, "cpu");
        assert_eq!(partitions[1].nodes_idle, 1);
        assert_eq!(partitions[2].name, "large");
        assert_eq!(partitions[2].nodes_idle, 0);
    }

    #[test]
    fn parse_sinfo_nodes_produces_synthetic_nodes() {
        let raw = "Partition gpu                     :      3 nodes idle\n\
                   Partition cpu                     :      0 nodes idle\n";
        let nodes = SlurmParser::parse_sinfo_nodes(raw);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "gpu");
        assert_eq!(nodes[0].state, "idle");
        assert_eq!(nodes[0].cpus_idle, 3);
        assert_eq!(nodes[1].name, "cpu");
        assert_eq!(nodes[1].state, "empty");
        assert_eq!(nodes[1].cpus_idle, 0);
    }

    #[test]
    fn parses_sacct_rows_and_success() {
        let raw = "1001|train|COMPLETED|0:0|00:30:12|2026-06-25T09:00:00|2026-06-25T09:30:12\n\
                   1002|eval|FAILED|1:0|00:00:05|2026-06-25T10:00:00|2026-06-25T10:00:05\n";
        let entries = SlurmParser::parse_sacct(raw);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].succeeded());
        assert!(!entries[1].succeeded());
        assert_eq!(entries[1].exit_code, "1:0");
    }

    #[test]
    fn detail_folds_maxrss_from_the_step_row() {
        // Allocation row has no MaxRSS; the .batch step carries 2072K.
        let raw = "2|acct_test_1|root|root|debug|node01|2|4Gn||00:00.002|COMPLETED|0:0|2026-06-25T02:22:47|2026-06-25T02:22:48|2026-06-25T02:22:53|00:00:05|/workspace\n\
                   2.batch|batch|||||2||2072K|00:00.002|COMPLETED|0:0|2026-06-25T02:22:48|2026-06-25T02:22:48|2026-06-25T02:22:53|00:00:05|\n";
        let d = SlurmParser::parse_sacct_detail(raw, "2").expect("allocation row present");
        assert_eq!(d.name, "acct_test_1");
        assert_eq!(d.alloc_cpus, "2");
        assert_eq!(d.req_mem, "4Gn");
        assert_eq!(d.work_dir, "/workspace");
        assert_eq!(d.max_rss.as_deref(), Some("2072K"));
    }

    #[test]
    fn detail_is_none_without_allocation_row() {
        let raw = "9.batch|batch|||||2||500K|00:00.001|COMPLETED|0:0|x|x|x|00:00:01|\n";
        assert!(SlurmParser::parse_sacct_detail(raw, "9").is_none());
    }

    #[test]
    fn mem_parsing_orders_units() {
        assert!(parse_mem_to_bytes("1G").unwrap() > parse_mem_to_bytes("900M").unwrap());
        assert!(parse_mem_to_bytes("2072K").unwrap() > parse_mem_to_bytes("1000").unwrap());
        assert_eq!(parse_mem_to_bytes(""), None);
        assert_eq!(parse_mem_to_bytes("0n"), None);
    }
}
